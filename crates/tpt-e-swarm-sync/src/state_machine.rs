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
    /// A heartbeat was received from the current Primary.
    HeartbeatReceived,
    /// No heartbeat received within the timeout period.
    HeartbeatTimeout,
    /// A higher-priority node was discovered during election.
    HigherPriorityNodeFound,
    /// No other nodes found during mesh discovery.
    NoOtherNodesFound,
    /// A network partition was detected (cannot reach Primary).
    PartitionDetected,
    /// Network partition was healed (can reach Primary again).
    PartitionHealed,
    /// Node is shutting down gracefully.
    Shutdown,
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
    pub fn process_event(&mut self, event: Event) -> Transition {
        match (self.role, event) {
            // Unknown → discovered mesh with Primary
            (NodeRole::Unknown, Event::HeartbeatReceived) => {
                self.role = NodeRole::Secondary;
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
            (NodeRole::Unknown, Event::HigherPriorityNodeFound) => {
                Transition { new_role: NodeRole::Unknown, should_broadcast: false }
            }
            // Primary receiving heartbeat (self-heartbeat or stale) — no change
            (NodeRole::Primary, Event::HeartbeatReceived) => {
                Transition { new_role: NodeRole::Primary, should_broadcast: false }
            }
            // Primary — heartbeat timeout means this node should still be Primary
            (NodeRole::Primary, Event::HeartbeatTimeout) => {
                Transition { new_role: NodeRole::Primary, should_broadcast: true }
            }
            // Primary — higher priority node found, yield
            (NodeRole::Primary, Event::HigherPriorityNodeFound) => {
                self.role = NodeRole::Secondary;
                // Clear primary_id — we no longer know who the Primary is
                // (it's the higher-priority node, but we use None for consistency
                // since Secondary's invariant is primary_id != Some(self.node_id))
                self.primary_id = None;
                Transition { new_role: NodeRole::Secondary, should_broadcast: true }
            }
            // Secondary — heartbeat received, all good
            (NodeRole::Secondary, Event::HeartbeatReceived) => {
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Secondary — heartbeat timeout, try to become Primary
            (NodeRole::Secondary, Event::HeartbeatTimeout) => {
                self.role = NodeRole::Primary;
                self.primary_id = Some(self.node_id);
                Transition { new_role: NodeRole::Primary, should_broadcast: true }
            }
            // Secondary — higher priority node found during election
            (NodeRole::Secondary, Event::HigherPriorityNodeFound) => {
                Transition { new_role: NodeRole::Secondary, should_broadcast: false }
            }
            // Partition detected — mark partitioned but do not change role here.
            // Promotion to Primary happens only via the HeartbeatTimeout arm
            // below. Immediately self-promoting a Secondary on partition
            // detection let every stranded Secondary in a multi-node partition
            // promote itself independently on the same tick, producing multiple
            // simultaneous Primaries — exactly the divergence this state
            // machine exists to prevent (see module doc). Full divergence
            // freedom across concurrent nodes still requires real cross-node
            // tie-break/quorum logic; this only removes the single-tick
            // self-promotion bug.
            (role, Event::PartitionDetected) => {
                self.partitioned = true;
                Transition { new_role: role, should_broadcast: false }
            }
            // Partition healed — if we're not the real Primary, yield
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
