//! Tests for the mesh state machine.

#![allow(missing_docs)]

use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

/// Test initial state is Unknown.
#[test]
fn initial_state_is_unknown() {
    let sm = MeshStateMachine::new(1);
    assert_eq!(sm.role(), NodeRole::Unknown);
    assert!(sm.is_consistent());
}

/// Test Unknown → Primary when no other nodes found.
#[test]
fn unknown_to_primary() {
    let mut sm = MeshStateMachine::new(1);
    let t = sm.process_event(Event::NoOtherNodesFound);
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(t.should_broadcast);
    assert_eq!(sm.role(), NodeRole::Primary);
    assert!(sm.is_consistent());
}

/// Test Unknown → Secondary when heartbeat received.
#[test]
fn unknown_to_secondary() {
    let mut sm = MeshStateMachine::new(1);
    let t = sm.process_event(Event::HeartbeatReceived { sender_id: 2 });
    assert_eq!(t.new_role, NodeRole::Secondary);
    assert!(!t.should_broadcast);
    assert_eq!(sm.role(), NodeRole::Secondary);
    assert!(sm.is_consistent());
}

/// Test Secondary → Primary on heartbeat timeout.
#[test]
fn secondary_to_primary_on_timeout() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::HeartbeatReceived { sender_id: 2 });
    let t = sm.process_event(Event::HeartbeatTimeout);
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(t.should_broadcast);
    assert!(sm.is_consistent());
}

/// Test Primary → Secondary on higher priority node (lower ID).
#[test]
fn primary_yields_to_higher_priority() {
    let mut sm = MeshStateMachine::new(2);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    let t = sm.process_event(Event::HigherPriorityNodeFound { candidate_id: 1 });
    assert_eq!(t.new_role, NodeRole::Secondary);
    assert!(t.should_broadcast);
    assert!(sm.is_consistent());
}

/// Test Primary ignores lower-priority candidate (higher ID).
#[test]
fn primary_ignores_lower_priority() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    let t = sm.process_event(Event::HigherPriorityNodeFound { candidate_id: 2 });
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(!t.should_broadcast);
    assert!(sm.is_consistent());
}

/// Test Primary ignores equal-priority candidate (same ID).
#[test]
fn primary_ignores_equal_priority() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    let t = sm.process_event(Event::HigherPriorityNodeFound { candidate_id: 1 });
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(!t.should_broadcast);
    assert!(sm.is_consistent());
}

/// Test Primary yields to lower-ID heartbeat after partition heal.
#[test]
fn primary_yields_to_lower_id_heartbeat() {
    let mut sm = MeshStateMachine::new(5);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    // Heartbeat from node 3 (lower ID = higher priority)
    let t = sm.process_event(Event::HeartbeatReceived { sender_id: 3 });
    assert_eq!(t.new_role, NodeRole::Secondary);
    assert!(sm.primary_id() == Some(3));
    assert!(sm.is_consistent());
}

/// Test Primary ignores higher-ID heartbeat.
#[test]
fn primary_ignores_higher_id_heartbeat() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    let t = sm.process_event(Event::HeartbeatReceived { sender_id: 5 });
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(!t.should_broadcast);
    assert!(sm.is_consistent());
}

/// Test partition detection does not itself promote a Secondary — only the
/// subsequent heartbeat timeout does.
#[test]
fn partition_detection_secondary() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::HeartbeatReceived { sender_id: 100 });
    let t = sm.process_event(Event::PartitionDetected);
    assert_eq!(t.new_role, NodeRole::Secondary);
    assert!(sm.is_consistent());

    let t = sm.process_event(Event::HeartbeatTimeout);
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(sm.is_consistent());
}

/// Test that multiple Secondaries stranded together do not simultaneously
/// self-promote on partition detection alone.
#[test]
fn partition_does_not_cause_simultaneous_dual_primary() {
    let mut a = MeshStateMachine::new(1);
    let mut b = MeshStateMachine::new(2);
    let _ = a.process_event(Event::HeartbeatReceived { sender_id: 100 });
    let _ = b.process_event(Event::HeartbeatReceived { sender_id: 100 });

    let ta = a.process_event(Event::PartitionDetected);
    let tb = b.process_event(Event::PartitionDetected);

    assert_ne!(ta.new_role, NodeRole::Primary);
    assert_ne!(tb.new_role, NodeRole::Primary);
    assert!(a.is_consistent());
    assert!(b.is_consistent());
}

/// Test partition healing.
#[test]
fn partition_healing() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    let _ = sm.process_event(Event::PartitionDetected);
    let t = sm.process_event(Event::PartitionHealed);
    assert_eq!(t.new_role, NodeRole::Primary);
    assert!(sm.is_consistent());
}

/// Test shutdown transitions to Unknown.
#[test]
fn shutdown_transitions_to_unknown() {
    let mut sm = MeshStateMachine::new(1);
    let _ = sm.process_event(Event::NoOtherNodesFound);
    let t = sm.process_event(Event::Shutdown);
    assert_eq!(t.new_role, NodeRole::Unknown);
    assert_eq!(sm.role(), NodeRole::Unknown);
    assert!(sm.is_consistent());
}

/// Test consistency invariant is maintained through all transitions.
#[test]
fn consistency_invariant_always_holds() {
    let mut sm = MeshStateMachine::new(1);
    let events = [
        Event::HeartbeatReceived { sender_id: 2 },
        Event::HeartbeatTimeout,
        Event::HigherPriorityNodeFound { candidate_id: 2 },
        Event::HeartbeatReceived { sender_id: 3 },
        Event::PartitionDetected,
        Event::PartitionHealed,
        Event::HeartbeatTimeout,
        Event::Shutdown,
    ];
    for event in events {
        let _ = sm.process_event(event);
        assert!(sm.is_consistent(), "Invariant broken after {:?}", event);
    }
}
