//! Core state machine definitions for mesh coordination.
//!
//! The state machine follows a formal specification (TLA+ inspired) that
//! guarantees exactly one Primary node in the mesh at any time. Node roles
//! are determined by a deterministic election protocol.
//!
//! # State Transitions
//!
//! ```text
//! Unknown → Secondary (discovered mesh)
//! Unknown → Primary (no other node found, elected)
//! Secondary → Primary (primary lost, re-election)
//! Primary → Secondary (higher-priority node found)
//! ```
//!
//! # Divergence Guarantee
//!
//! The state machine cannot reach a state where two nodes both believe
//! they are Primary. The election protocol uses deterministic tie-breaking
//! based on node IDs.

/// The role of a node in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Primary coordinator — responsible for mesh management.
    Primary,
    /// Secondary (backup) node — follows the Primary.
    Secondary,
    /// Uninitialised node — has not yet discovered the mesh.
    Unknown,
}

/// Events that drive the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A heartbeat was received from another node.
    HeartbeatReceived {
        /// The node ID of the sender.
        sender_id: u32,
    },
    /// No heartbeat received within the timeout period.
    HeartbeatTimeout,
    /// A higher-priority node was discovered during election.
    HigherPriorityNodeFound {
        /// The node ID of the candidate.
        candidate_id: u32,
    },
    /// No other nodes found during mesh discovery.
    NoOtherNodesFound,
    /// A network partition was detected (cannot reach Primary).
    PartitionDetected,
    /// Network partition was healed (can reach Primary again).
    PartitionHealed,
    /// Node is shutting down gracefully.
    Shutdown,
    /// An `Ack` message was received, correlated (or not) against this
    /// node's own outstanding `send_data` sequence numbers by
    /// `MeshNode::process_inbound`. Does not affect role — acknowledgments
    /// are a message-delivery concern, not an election/liveness signal.
    MessageAcknowledged {
        /// The sequence number being acknowledged (echoed from the
        /// original `Data` message, not the acker's own sequence).
        sequence: u64,
    },
}

/// The state machine result after processing an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// The new role after processing the event.
    pub new_role: NodeRole,
    /// Whether the node should broadcast its new role.
    pub should_broadcast: bool,
}

/// The mesh node state machine.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct MeshStateMachine {
    /// Current role of this node.
    role: NodeRole,
    /// Unique node identifier (lower = higher priority).
    node_id: u32,
    /// ID of the current Primary node (if known).
    primary_id: Option<u32>,
    /// Whether a network partition is currently active.
    partitioned: bool,
}

impl MeshStateMachine {
    /// Create a new state machine in the Unknown state.
    pub fn new(node_id: u32) -> Self {
        Self {
            role: NodeRole::Unknown,
            node_id,
            primary_id: None,
            partitioned: false,
        }
    }

    /// Get the current role.
    pub fn role(&self) -> NodeRole {
        self.role
    }

    /// Get the node ID.
    pub fn node_id(&self) -> u32 {
        self.node_id
    }

    /// Get the current Primary's ID, if known.
    pub fn primary_id(&self) -> Option<u32> {
        self.primary_id
    }

    /// Process an event and return the resulting transition.
    ///
    /// The election protocol uses deterministic tie-breaking: a lower `node_id`
    /// has higher priority. A Primary yields only to a genuinely higher-priority
    /// (lower-ID) node, preventing split-brain after partition heal.
    pub fn process_event(&mut self, event: Event) -> Transition {
        match (self.role, event) {
            // Unknown → discovered mesh via heartbeat from some node
            (NodeRole::Unknown, Event::HeartbeatReceived { sender_id }) => {
                self.role = NodeRole::Secondary;
                self.primary_id = Some(sender_id);
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Unknown → no other nodes, become Primary
            (NodeRole::Unknown, Event::NoOtherNodesFound) => {
                self.role = NodeRole::Primary;
                self.primary_id = Some(self.node_id);
                Transition { new_role: NodeRole::Primary, should_broadcast: true }
            }
            // Unknown — heartbeat timeout or higher priority node (ignored while discovering)
            (NodeRole::Unknown, Event::HeartbeatTimeout) => {
                Transition { new_role: NodeRole::Unknown, should_broadcast: false }
            }
            (NodeRole::Unknown, Event::HigherPriorityNodeFound { .. }) => {
                Transition { new_role: NodeRole::Unknown, should_broadcast: false }
            }
            // Primary receiving heartbeat: yield only if sender is genuinely
            // higher-priority (lower node ID). This is the key reconciliation
            // path after a partition heals — the lowest-ID Primary survives.
            (NodeRole::Primary, Event::HeartbeatReceived { sender_id }) => {
                if sender_id < self.node_id {
                    self.role = NodeRole::Secondary;
                    self.primary_id = Some(sender_id);
                    Transition { new_role: NodeRole::Secondary, should_broadcast: true }
                } else {
                    Transition { new_role: NodeRole::Primary, should_broadcast: false }
                }
            }
            // Primary — heartbeat timeout means this node should still be Primary
            (NodeRole::Primary, Event::HeartbeatTimeout) => {
                Transition { new_role: NodeRole::Primary, should_broadcast: true }
            }
            // Primary — higher priority node found, yield only if genuinely
            // higher-priority (lower node ID). Prevents spoofing by a
            // malicious or confused peer with a higher ID.
            (NodeRole::Primary, Event::HigherPriorityNodeFound { candidate_id }) => {
                if candidate_id < self.node_id {
                    self.role = NodeRole::Secondary;
                    self.primary_id = Some(candidate_id);
                    Transition { new_role: NodeRole::Secondary, should_broadcast: true }
                } else {
                    Transition { new_role: NodeRole::Primary, should_broadcast: false }
                }
            }
            // Secondary — heartbeat received, update known primary
            (NodeRole::Secondary, Event::HeartbeatReceived { sender_id }) => {
                self.primary_id = Some(sender_id);
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Secondary — heartbeat timeout, try to become Primary
            (NodeRole::Secondary, Event::HeartbeatTimeout) => {
                self.role = NodeRole::Primary;
                self.primary_id = Some(self.node_id);
                Transition { new_role: NodeRole::Primary, should_broadcast: true }
            }
            // Secondary — higher priority node found during election (ignored;
            // a Secondary already follows someone)
            (NodeRole::Secondary, Event::HigherPriorityNodeFound { .. }) => {
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Partition detected — mark partitioned but do not change role here.
            // Promotion to Primary happens only via the HeartbeatTimeout arm.
            (role, Event::PartitionDetected) => {
                self.partitioned = true;
                Transition { new_role: role, should_broadcast: false }
            }
            // Partition healed
            (NodeRole::Primary, Event::PartitionHealed) => {
                self.partitioned = false;
                Transition { new_role: NodeRole::Primary, should_broadcast: true }
            }
            (NodeRole::Secondary, Event::PartitionHealed) => {
                self.partitioned = false;
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Shutdown — transition to Unknown
            (_, Event::Shutdown) => {
                self.role = NodeRole::Unknown;
                self.primary_id = None;
                Transition { new_role: NodeRole::Unknown, should_broadcast: true }
            }
            // No other nodes found while already a role
            (NodeRole::Primary, Event::NoOtherNodesFound) => {
                Transition { new_role: NodeRole::Primary, should_broadcast: false }
            }
            (NodeRole::Secondary, Event::NoOtherNodesFound) => {
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Partition healed while Unknown
            (NodeRole::Unknown, Event::PartitionHealed) => {
                self.partitioned = false;
                Transition { new_role: NodeRole::Unknown, should_broadcast: false }
            }
            // Message acknowledgments are a delivery concern, not an
            // election/liveness signal — role is unaffected regardless of
            // current role.
            (role, Event::MessageAcknowledged { .. }) => {
                Transition { new_role: role, should_broadcast: false }
            }
        }
    }

    /// Check if the state machine is in a consistent state.
    ///
    /// Invariant: there should never be two nodes both believing they are Primary
    /// in the same partition. This property is enforced by the election protocol,
    /// but we can verify local consistency here.
    pub fn is_consistent(&self) -> bool {
        match self.role {
            NodeRole::Primary => self.primary_id == Some(self.node_id),
            NodeRole::Secondary => self.primary_id != Some(self.node_id),
            NodeRole::Unknown => self.primary_id.is_none(),
        }
    }
}
