//! Integration test for mesh node with tpt-e-chronos ring buffer.

#![cfg(feature = "mock")]
#![allow(missing_docs)]

use tpt_e_swarm_sync::mesh::{Message, MessageType, MeshNode};
use tpt_e_swarm_sync::state_machine::{Event, NodeRole};

/// Test that a mesh node can be created and starts in Unknown state.
#[test]
fn mesh_node_initial_state() {
    let node = MeshNode::new(1);
    assert_eq!(node.role(), NodeRole::Unknown);
    assert!(node.is_consistent());
}

/// Test that a mesh node can receive and process messages.
#[test]
fn mesh_node_message_processing() {
    let mut node = MeshNode::new(1);
    let msg = Message {
        sender_id: 42,
        sequence: 1,
        msg_type: MessageType::Heartbeat,
        payload: [0xAB; 256],
    };
    assert!(node.receive_message(msg).is_ok());
    let event = node.process_inbound();
    assert_eq!(event, Some(Event::HeartbeatReceived { sender_id: 42 }));
}

/// Test that a mesh node can send and receive messages.
#[test]
fn mesh_node_send_receive() {
    let node = MeshNode::new(1);
    let msg = Message {
        sender_id: 2,
        sequence: 1,
        msg_type: MessageType::Heartbeat,
        payload: [0xCD; 256],
    };
    assert!(node.send_message(msg).is_ok());
    let out = node.next_outbound();
    assert!(out.is_some());
    assert_eq!(out.unwrap().sequence, 1);
}

/// Test that a mesh node processes events through the state machine.
#[test]
fn mesh_node_event_processing() {
    let mut node = MeshNode::new(1);
    let t = node.process_event(Event::NoOtherNodesFound);
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(node.is_consistent());
}

/// Test that the ring buffer integration works under load.
#[test]
fn mesh_node_ring_buffer_under_load() {
    let node = MeshNode::new(1);
    // Fill the outbound buffer
    for i in 0..32u64 {
        let msg = Message {
            sender_id: 1,
            sequence: i,
            msg_type: MessageType::Data,
            payload: [i as u8; 256],
        };
        assert!(node.send_message(msg).is_ok());
    }
    // Buffer should be full now
    let msg = Message {
        sender_id: 1,
        sequence: 32,
        msg_type: MessageType::Data,
        payload: [32u8; 256],
    };
    assert!(node.send_message(msg).is_err());
    // Drain
    for _ in 0..32 {
        assert!(node.next_outbound().is_some());
    }
    assert!(node.next_outbound().is_none());
}
