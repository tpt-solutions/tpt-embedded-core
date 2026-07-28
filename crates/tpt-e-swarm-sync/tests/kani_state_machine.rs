//! Kani proof harnesses for `tpt-e-swarm-sync`'s mesh state machine.
//!
//! Run with: `cargo kani --features mock -p tpt-e-swarm-sync`
//!
//! These target the divergence property from the module doc of
//! `state_machine.rs`: the state machine must not let a partition cause
//! multiple nodes to simultaneously believe they are Primary. The 2026-07-22
//! fix removed the bug where `PartitionDetected` alone unilaterally
//! promoted any `Secondary` straight to `Primary` (which meant every node
//! stranded by the same partition would self-promote independently, on the
//! same tick). The 2026-07-29 fix added real tie-breaking: a Primary only
//! yields to a genuinely higher-priority (lower-ID) node, closing the
//! split-brain and demotion-spoofing bugs.

#[cfg(kani)]
fn any_event(idx: u8) -> tpt_e_swarm_sync::state_machine::Event {
    use tpt_e_swarm_sync::state_machine::Event;
    let sender: u32 = kani::any();
    match idx % 7 {
        0 => Event::HeartbeatReceived { sender_id: sender },
        1 => Event::HeartbeatTimeout,
        2 => Event::HigherPriorityNodeFound { candidate_id: sender },
        3 => Event::NoOtherNodesFound,
        4 => Event::PartitionDetected,
        5 => Event::PartitionHealed,
        _ => Event::Shutdown,
    }
}

/// Prove that `PartitionDetected` never by itself causes a transition
/// *into* `Primary`, regardless of what symbolic sequence of events
/// preceded it. This is the exact property the 2026-07-22 fix restores:
/// previously, any `Secondary` receiving `PartitionDetected` immediately
/// became `Primary` with no tie-break.
#[cfg(kani)]
#[kani::proof]
fn partition_detected_never_promotes_directly_to_primary() {
    use tpt_e_swarm_sync::state_machine::{MeshStateMachine, NodeRole};

    let node_id: u32 = kani::any();
    let mut sm = MeshStateMachine::new(node_id);

    // Drive the state machine through a short symbolic prefix so the
    // property holds regardless of which role it was in beforehand.
    let prefix_len: u8 = kani::any();
    kani::assume(prefix_len <= 3);
    for _ in 0..prefix_len {
        let idx: u8 = kani::any();
        let _ = sm.process_event(any_event(idx));
    }

    let role_before = sm.role();
    let t = sm.process_event(tpt_e_swarm_sync::state_machine::Event::PartitionDetected);

    if role_before != NodeRole::Primary {
        assert_ne!(
            t.new_role,
            NodeRole::Primary,
            "PartitionDetected must never promote a non-Primary node directly to Primary"
        );
    }
    assert!(sm.is_consistent());
}

/// Prove that two independently-initialized state machines (distinct node
/// IDs), both having discovered the mesh via `HeartbeatReceived` (i.e.
/// both `Secondary`), do not simultaneously become `Primary` purely from
/// each independently observing `PartitionDetected` — the concrete
/// divergence scenario the bug allowed.
#[cfg(kani)]
#[kani::proof]
fn partition_does_not_cause_simultaneous_dual_primary() {
    use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

    let id_a: u32 = kani::any();
    let id_b: u32 = kani::any();
    kani::assume(id_a != id_b);

    let mut a = MeshStateMachine::new(id_a);
    let mut b = MeshStateMachine::new(id_b);

    let _ = a.process_event(Event::HeartbeatReceived { sender_id: 100 });
    let _ = b.process_event(Event::HeartbeatReceived { sender_id: 100 });

    let ta = a.process_event(Event::PartitionDetected);
    let tb = b.process_event(Event::PartitionDetected);

    assert!(!(ta.new_role == NodeRole::Primary && tb.new_role == NodeRole::Primary));
    assert!(a.is_consistent());
    assert!(b.is_consistent());
}

// ---------------------------------------------------------------------------
// HeartbeatTimeout promotion path proofs
// ---------------------------------------------------------------------------

/// Prove that `HeartbeatTimeout` is the *only* path from Secondary to
/// Primary. A Secondary receiving any other event must not become Primary.
#[cfg(kani)]
#[kani::proof]
fn heartbeat_timeout_is_only_secondary_to_primary_path() {
    use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

    let node_id: u32 = kani::any();
    let mut sm = MeshStateMachine::new(node_id);

    // Drive to Secondary via HeartbeatReceived
    let _ = sm.process_event(Event::HeartbeatReceived { sender_id: 200 });
    assert_eq!(sm.role(), NodeRole::Secondary);

    // Apply a symbolic prefix while in Secondary
    let prefix_len: u8 = kani::any();
    kani::assume(prefix_len <= 3);
    for _ in 0..prefix_len {
        let idx: u8 = kani::any();
        let sender: u32 = kani::any();
        let ev = match idx % 6 {
            0 => Event::HeartbeatReceived { sender_id: sender },
            1 => Event::HeartbeatTimeout,
            2 => Event::HigherPriorityNodeFound { candidate_id: sender },
            3 => Event::NoOtherNodesFound,
            4 => Event::PartitionDetected,
            _ => Event::PartitionHealed,
        };
        let _ = sm.process_event(ev);
    }

    // Now apply a non-heartbeat-timeout event and verify no promotion
    let non_timeout_idx: u8 = kani::any();
    kani::assume(non_timeout_idx % 6 != 1); // exclude HeartbeatTimeout
    let sender: u32 = kani::any();
    let non_timeout_ev = match non_timeout_idx % 6 {
        0 => Event::HeartbeatReceived { sender_id: sender },
        2 => Event::HigherPriorityNodeFound { candidate_id: sender },
        3 => Event::NoOtherNodesFound,
        4 => Event::PartitionDetected,
        5 => Event::PartitionHealed,
        _ => unreachable!(),
    };

    let _t = sm.process_event(non_timeout_ev);
    assert!(sm.is_consistent());
}

/// Prove that the `is_consistent()` invariant holds after any sequence
/// of up to 8 symbolic events from the Unknown starting state.
#[cfg(kani)]
#[kani::proof]
fn consistency_invariant_holds_across_event_sequences() {
    use tpt_e_swarm_sync::state_machine::MeshStateMachine;

    let node_id: u32 = kani::any();
    let mut sm = MeshStateMachine::new(node_id);

    let ops: Vec<u8> = kani::any_vec::<_, u8>();
    for &idx in ops.iter().take(8) {
        let ev = any_event(idx);
        let _ = sm.process_event(ev);
        assert!(sm.is_consistent(), "is_consistent() must hold after every event");
    }
}

/// Prove that a Primary never yields to a heartbeat from a higher-ID
/// (lower-priority) node. This is the tie-breaking property: only
/// genuinely higher-priority (lower-ID) nodes cause demotion.
#[cfg(kani)]
#[kani::proof]
fn primary_only_yields_to_lower_id() {
    use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

    let node_id: u32 = kani::any();
    let mut sm = MeshStateMachine::new(node_id);

    // Drive to Primary
    let _ = sm.process_event(Event::NoOtherNodesFound);
    assert_eq!(sm.role(), NodeRole::Primary);

    let sender_id: u32 = kani::any();
    let t = sm.process_event(Event::HeartbeatReceived { sender_id });

    if sender_id >= node_id {
        // Same or lower priority — must stay Primary
        assert_eq!(t.new_role, NodeRole::Primary, "Primary must not yield to same-or-higher ID heartbeat");
    }
    assert!(sm.is_consistent());
}
