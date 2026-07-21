//! Mesh network message types and sequencing.
//!
//! Messages are queued via `tpt-e-chronos` ring buffers for deterministic,
//! panic-free message handling during network events.

use crate::state_machine::Event;

/// A message exchanged between mesh nodes.
#[derive(Debug, Copy, Clone)]
pub struct Message {
    /// Monotonically increasing sequence number.
    pub sequence: u64,
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
}

impl MeshNode {
    /// Create a new mesh node with the given ID.
    pub fn new(node_id: u32) -> Self {
        Self {
            state_machine: crate::state_machine::MeshStateMachine::new(node_id),
            outbound: tpt_e_chronos::ring_buf::RingBuf::new(Message {
                sequence: 0,
                payload: [0u8; 256],
            }),
            inbound: tpt_e_chronos::ring_buf::RingBuf::new(Message {
                sequence: 0,
                payload: [0u8; 256],
            }),
        }
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
    pub fn process_inbound(&mut self) -> Option<crate::state_machine::Event> {
        self.inbound.pop().map(|msg| {
            // For now, any received message is treated as a heartbeat.
            // A real implementation would inspect the message type.
            let _ = msg;
            Event::HeartbeatReceived
        })
    }

    /// Push an outbound message to the queue.
    pub fn send_message(&self, msg: Message) -> Result<(), Message> {
        self.outbound.push(msg)
    }

    /// Dequeue the next outbound message for transmission.
    pub fn next_outbound(&self) -> Option<Message> {
        self.outbound.pop()
    }

    /// Process an event through the state machine.
    pub fn process_event(&mut self, event: Event) -> crate::state_machine::Transition {
        self.state_machine.process_event(event)
    }

    /// Check if the node is in a consistent state.
    pub fn is_consistent(&self) -> bool {
        self.state_machine.is_consistent()
    }
}
