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
//! same tick). These harnesses are the regression guard for that fix.
//!
//! Full divergence-freedom across concurrent nodes over arbitrary event
//! interleavings still requires real cross-node tie-break/quorum logic
//! that does not exist yet (see `todo.md`) — these harnesses prove the
//! narrower, currently-true property that motivated the fix, not the full
//! cross-node guarantee the module doc aspires to.

#[cfg(kani)]
fn any_event(idx: u8) -> tpt_e_swarm_sync::state_machine::Event {
    use tpt_e_swarm_sync::state_machine::Event;
    match idx % 7 {
        0 => Event::HeartbeatReceived,
        1 => Event::HeartbeatTimeout,
        2 => Event::HigherPriorityNodeFound,
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

    let _ = a.process_event(Event::HeartbeatReceived);
    let _ = b.process_event(Event::HeartbeatReceived);

    let ta = a.process_event(Event::PartitionDetected);
    let tb = b.process_event(Event::PartitionDetected);

    assert!(!(ta.new_role == NodeRole::Primary && tb.new_role == NodeRole::Primary));
    assert!(a.is_consistent());
    assert!(b.is_consistent());
}
