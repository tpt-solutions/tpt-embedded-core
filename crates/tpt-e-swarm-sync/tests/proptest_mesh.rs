//! Proptest for the mesh state machine.

#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

/// Simple PRNG for generating random events in proptest.
fn simple_rng(seed: u32) -> Event {
    let sender = seed.wrapping_mul(7).wrapping_add(1); // deterministic sender ID from seed
    match seed % 7 {
        0 => Event::HeartbeatReceived { sender_id: sender },
        1 => Event::HeartbeatTimeout,
        2 => Event::HigherPriorityNodeFound { candidate_id: sender },
        3 => Event::NoOtherNodesFound,
        4 => Event::PartitionDetected,
        5 => Event::PartitionHealed,
        _ => Event::Shutdown,
    }
}

/// Generate a vector of events from a seed sequence for proptest.
fn events_from_seeds(seeds: Vec<u32>) -> Vec<Event> {
    seeds.into_iter().map(simple_rng).collect()
}

proptest! {
    #[test]
    fn consistency_invariant_always_holds(seeds: Vec<u32>) {
        let events = events_from_seeds(seeds);
        let mut sm = MeshStateMachine::new(1);
        for event in events {
            let _ = sm.process_event(event);
            prop_assert!(sm.is_consistent(), "Invariant broken after {:?}", event);
        }
    }

    #[test]
    fn role_is_always_one_of_three(seeds: Vec<u32>) {
        let events = events_from_seeds(seeds);
        let mut sm = MeshStateMachine::new(1);
        for event in events {
            let t = sm.process_event(event);
            prop_assert!(
                t.new_role == NodeRole::Primary
                    || t.new_role == NodeRole::Secondary
                    || t.new_role == NodeRole::Unknown,
                "Invalid role: {:?}", t.new_role
            );
        }
    }

    #[test]
    fn node_id_preserved(node_id in 0u32..u32::MAX, seeds: Vec<u32>) {
        let events = events_from_seeds(seeds);
        let mut sm = MeshStateMachine::new(node_id);
        for event in events {
            let _ = sm.process_event(event);
        }
        prop_assert_eq!(sm.node_id(), node_id);
    }

    #[test]
    fn unknown_always_becomes_valid_role(seeds: Vec<u32>) {
        let events = events_from_seeds(seeds);
        let mut sm = MeshStateMachine::new(1);
        for event in events {
            let _ = sm.process_event(event);
        }
        let role = sm.role();
        prop_assert!(
            role == NodeRole::Primary || role == NodeRole::Secondary || role == NodeRole::Unknown,
            "Invalid final role: {:?}", role
        );
    }
}
