//! Mesh network message types and sequencing.
//!
//! Messages are queued via `tpt-e-chronos` ring buffers for deterministic,
//! panic-free message handling during network events.

/// The type of a mesh message, carried in [`Message::msg_type`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Heartbeat — periodic liveness signal from the Primary.
    Heartbeat = 0,
    /// Election — node is announcing candidacy or contesting.
    Election = 1,
    /// Acknowledgment — confirms receipt of a previous message.
    Ack = 2,
    /// Data — application-level payload.
    Data = 3,
}

/// Maximum number of outstanding (sent, not yet acknowledged) `Data`
/// message sequence numbers a `MeshNode` tracks at once. Bounded and
/// fixed-capacity, matching this crate's no-alloc, WCET-friendly design —
/// once full, `send_data` still sends the message but stops tracking it for
/// acknowledgment (see `MeshNode::send_data`).
const MAX_PENDING_ACKS: usize = 16;

impl MessageType {
    /// Convert from raw byte, returning `None` for unknown types.
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Heartbeat),
            1 => Some(Self::Election),
            2 => Some(Self::Ack),
            3 => Some(Self::Data),
            _ => None,
        }
    }
}

/// A message exchanged between mesh nodes.
#[derive(Debug, Copy, Clone)]
pub struct Message {
    /// Node ID of the message sender.
    pub sender_id: u32,
    /// Monotonically increasing sequence number.
    pub sequence: u64,
    /// Message type (encoded in the first byte of `payload`).
    pub msg_type: MessageType,
    /// Message payload bytes.
    pub payload: [u8; 256],
}

/// A mesh node that coordinates with other nodes via message passing.
///
/// Uses `tpt-e-chronos` ring buffers for deterministic message queuing.
#[derive(Debug)]
pub struct MeshNode {
    /// The state machine governing this node's role.
    state_machine: crate::state_machine::MeshStateMachine,
    /// Outbound message queue (produced by state machine, consumed by network driver).
    outbound: tpt_e_chronos::ring_buf::RingBuf<Message, 32>,
    /// Inbound message queue (produced by network driver, consumed by state machine).
    inbound: tpt_e_chronos::ring_buf::RingBuf<Message, 32>,
    /// Monotonically increasing sequence counter for this node's own
    /// outbound messages. Real per-node state — not derived from the node
    /// ID (a message's `sequence` and `sender_id` are independent fields).
    next_sequence: u64,
    /// Sequence numbers of `Data` messages sent via [`Self::send_data`]
    /// that have not yet been acknowledged. Checked (and pruned) against
    /// inbound `Ack` messages in [`Self::process_inbound`].
    pending_acks: heapless::Vec<u64, MAX_PENDING_ACKS>,
}

impl MeshNode {
    /// Create a new mesh node with the given ID.
    pub fn new(node_id: u32) -> Self {
        Self {
            state_machine: crate::state_machine::MeshStateMachine::new(node_id),
            outbound: tpt_e_chronos::ring_buf::RingBuf::new(Message {
                sender_id: 0,
                sequence: 0,
                msg_type: MessageType::Heartbeat,
                payload: [0u8; 256],
            }),
            inbound: tpt_e_chronos::ring_buf::RingBuf::new(Message {
                sender_id: 0,
                sequence: 0,
                msg_type: MessageType::Heartbeat,
                payload: [0u8; 256],
            }),
            next_sequence: 0,
            pending_acks: heapless::Vec::new(),
        }
    }

    /// Allocate and return the next outbound sequence number for this node,
    /// advancing the counter. Used both by [`Self::send_data`] and by
    /// network drivers constructing other message types (e.g. heartbeats)
    /// on this node's behalf.
    pub fn next_outbound_sequence(&mut self) -> u64 {
        let seq = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        seq
    }

    /// Get the current role of this node.
    pub fn role(&self) -> crate::state_machine::NodeRole {
        self.state_machine.role()
    }

    /// Get the node ID.
    pub fn node_id(&self) -> u32 {
        self.state_machine.node_id()
    }

    /// Enqueue an inbound message from the network.
    pub fn receive_message(&self, msg: Message) -> Result<(), Message> {
        self.inbound.push(msg)
    }

    /// Process the next inbound message and return any event it generates.
    ///
    /// `Data` messages are auto-acknowledged: an `Ack` echoing the received
    /// message's sequence number is queued on `outbound` as a side effect
    /// (best-effort — if `outbound` is full, the ack is silently dropped,
    /// same as any other outbound-queue-full case elsewhere in this crate;
    /// the sender's `send_data` timing out un-acked is the recovery path,
    /// not implemented here — see the still-open reliability items in the
    /// root `todo.md`). `Ack` messages are correlated against
    /// `pending_acks` (removed if found) and surface as
    /// `Event::MessageAcknowledged`, distinct from `HeartbeatReceived` —
    /// unlike before, an ack is no longer indistinguishable from a
    /// heartbeat to the state machine.
    pub fn process_inbound(&mut self) -> Option<crate::state_machine::Event> {
        self.inbound.pop().map(|msg| match msg.msg_type {
            MessageType::Heartbeat => crate::state_machine::Event::HeartbeatReceived {
                sender_id: msg.sender_id,
            },
            MessageType::Election => crate::state_machine::Event::HigherPriorityNodeFound {
                candidate_id: msg.sender_id,
            },
            MessageType::Data => {
                let ack = Message {
                    sender_id: self.node_id(),
                    sequence: msg.sequence,
                    msg_type: MessageType::Ack,
                    payload: [0u8; 256],
                };
                let _ = self.outbound.push(ack);
                crate::state_machine::Event::HeartbeatReceived {
                    sender_id: msg.sender_id,
                }
            }
            MessageType::Ack => {
                if let Some(pos) = self.pending_acks.iter().position(|&s| s == msg.sequence) {
                    let _ = self.pending_acks.swap_remove(pos);
                }
                crate::state_machine::Event::MessageAcknowledged {
                    sequence: msg.sequence,
                }
            }
        })
    }

    /// Push an outbound message to the queue.
    pub fn send_message(&self, msg: Message) -> Result<(), Message> {
        self.outbound.push(msg)
    }

    /// Send a `Data` message, assigning it a real sequence number and
    /// tracking it as awaiting acknowledgment.
    ///
    /// Returns the assigned sequence number on success, or the message back
    /// (as [`Message::send_message`] does) if `outbound` is full.
    ///
    /// If `pending_acks` is already at capacity ([`MAX_PENDING_ACKS`]), the
    /// message is still sent, but is not tracked for acknowledgment — a
    /// bounded, WCET-friendly degradation (drop tracking, not the send)
    /// rather than an unbounded pending-ack list.
    pub fn send_data(&mut self, payload: [u8; 256]) -> Result<u64, Message> {
        let sequence = self.next_outbound_sequence();
        let msg = Message {
            sender_id: self.node_id(),
            sequence,
            msg_type: MessageType::Data,
            payload,
        };
        self.outbound.push(msg)?;
        // Best-effort: if the pending-acks tracker is full, the message is
        // still sent (above) but its ack will go uncorrelated.
        let _ = self.pending_acks.push(sequence);
        Ok(sequence)
    }

    /// Sequence numbers of `Data` messages sent via [`Self::send_data`]
    /// that have not yet been acknowledged.
    pub fn pending_ack_count(&self) -> usize {
        self.pending_acks.len()
    }

    /// Dequeue the next outbound message for transmission.
    pub fn next_outbound(&self) -> Option<Message> {
        self.outbound.pop()
    }

    /// Process an event through the state machine.
    pub fn process_event(&mut self, event: crate::state_machine::Event) -> crate::state_machine::Transition {
        self.state_machine.process_event(event)
    }

    /// Check if the node is in a consistent state.
    pub fn is_consistent(&self) -> bool {
        self.state_machine.is_consistent()
    }
}
