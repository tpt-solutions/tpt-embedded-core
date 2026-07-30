//! Tests for real message sequencing and ack correlation (2026-07-30).
//!
//! Before this, `Message.sequence` was set to the sender's node ID (not a
//! real counter) and `Ack` messages were indistinguishable from heartbeats
//! to the state machine — see the root `todo.md` for the full history.

#![cfg(feature = "mock")]
#![allow(missing_docs)]

use tpt_e_swarm_sync::mesh::{Message, MessageType, MeshNode};
use tpt_e_swarm_sync::mock::SimulatedNetwork;
use tpt_e_swarm_sync::state_machine::Event;

#[test]
fn send_data_assigns_increasing_sequence_numbers() {
    let mut node = MeshNode::new(1);
    let seq0 = node.send_data([0u8; 256]).expect("outbound not full");
    let seq1 = node.send_data([1u8; 256]).expect("outbound not full");
    let seq2 = node.send_data([2u8; 256]).expect("outbound not full");

    assert_eq!(seq0, 0);
    assert_eq!(seq1, 1);
    assert_eq!(seq2, 2);
}

#[test]
fn send_data_tracks_pending_ack() {
    let mut node = MeshNode::new(1);
    assert_eq!(node.pending_ack_count(), 0);

    let _ = node.send_data([0u8; 256]).expect("outbound not full");
    assert_eq!(node.pending_ack_count(), 1);

    let _ = node.send_data([1u8; 256]).expect("outbound not full");
    assert_eq!(node.pending_ack_count(), 2);
}

#[test]
fn receiving_data_auto_enqueues_an_ack() {
    let mut node = MeshNode::new(1);
    let msg = Message {
        sender_id: 2,
        sequence: 7,
        msg_type: MessageType::Data,
        payload: [0u8; 256],
    };
    assert!(node.receive_message(msg).is_ok());

    let event = node.process_inbound();
    assert_eq!(event, Some(Event::HeartbeatReceived { sender_id: 2 }));

    let outbound = node.next_outbound().expect("an Ack should have been queued");
    assert_eq!(outbound.msg_type, MessageType::Ack);
    assert_eq!(outbound.sequence, 7, "Ack must echo the acknowledged sequence, not a fresh one");
    assert_eq!(outbound.sender_id, 1);
}

#[test]
fn receiving_ack_correlates_and_clears_pending() {
    let mut node = MeshNode::new(1);
    let seq = node.send_data([0u8; 256]).expect("outbound not full");
    assert_eq!(node.pending_ack_count(), 1);
    let _ = node.next_outbound(); // drain the Data message itself

    let ack = Message {
        sender_id: 2,
        sequence: seq,
        msg_type: MessageType::Ack,
        payload: [0u8; 256],
    };
    assert!(node.receive_message(ack).is_ok());

    let event = node.process_inbound();
    assert_eq!(event, Some(Event::MessageAcknowledged { sequence: seq }));
    assert_eq!(
        node.pending_ack_count(),
        0,
        "the correlated ack must remove the sequence from pending_acks"
    );
}

#[test]
fn unmatched_ack_still_produces_event_but_does_not_underflow_pending() {
    let mut node = MeshNode::new(1);
    assert_eq!(node.pending_ack_count(), 0);

    let stray_ack = Message {
        sender_id: 2,
        sequence: 999,
        msg_type: MessageType::Ack,
        payload: [0u8; 256],
    };
    assert!(node.receive_message(stray_ack).is_ok());

    let event = node.process_inbound();
    assert_eq!(event, Some(Event::MessageAcknowledged { sequence: 999 }));
    assert_eq!(node.pending_ack_count(), 0);
}

#[test]
fn ack_event_does_not_change_role() {
    let mut node = MeshNode::new(1);
    let t = node.process_event(Event::NoOtherNodesFound);
    assert_eq!(t.new_role, tpt_e_swarm_sync::state_machine::NodeRole::Primary);

    let t = node.process_event(Event::MessageAcknowledged { sequence: 42 });
    assert_eq!(t.new_role, tpt_e_swarm_sync::state_machine::NodeRole::Primary);
    assert!(!t.should_broadcast);
}

#[test]
fn simulated_network_heartbeat_advances_real_sequence_counter() {
    let mut net = SimulatedNetwork::new();
    net.add_node(5); // deliberately not 0/1, to prove sequence != node_id
    net.add_node(9);

    net.send_heartbeat(5, 9);
    net.send_heartbeat(5, 9);
    net.send_heartbeat(5, 9);

    // The three heartbeats above must have advanced node 5's *real*
    // sequence counter (shared with `send_data`), not the old behavior of
    // always stamping `sequence: from as u64` (a constant 5, every time).
    // If the counter is real and shared, the next `send_data` on the same
    // node continues from where the heartbeats left off: sequence 3.
    let node5 = net.node_mut(5).expect("node 5 exists");
    let seq = node5.send_data([0u8; 256]).expect("outbound not full");
    assert_eq!(
        seq, 3,
        "heartbeat sends must consume the same real, monotonic sequence counter as send_data"
    );
}
