//! Multi-node election integration tests using `SimulatedNetwork`.
//!
//! These tests exercise the full mesh coordination path: message delivery,
//! state machine transitions, and convergence to a single Primary — all
//! via the simulated network harness (no hardware required).

#![cfg(feature = "mock")]
#![allow(missing_docs)]

use tpt_e_swarm_sync::mock::SimulatedNetwork;
use tpt_e_swarm_sync::state_machine::NodeRole;

/// Two nodes discover each other: exactly one becomes Primary.
#[test]
fn two_node_election_converges() {
    let mut net = SimulatedNetwork::new();
    net.add_node(1);
    net.add_node(2);

    // Both start Unknown
    assert_eq!(net.node(1).unwrap().role(), NodeRole::Unknown);
    assert_eq!(net.node(2).unwrap().role(), NodeRole::Unknown);

    // Node 1 becomes Primary (no other nodes found during its election)
    let t = net.node_mut(1).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::NoOtherNodesFound,
    );
    assert_eq!(t.new_role, NodeRole::Primary);

    // Deliver heartbeat from node 1 → node 2
    net.send_heartbeat(1, 2);
    net.deliver_all();

    // Node 2 should be Secondary (discovered an existing Primary)
    assert_eq!(net.node(2).unwrap().role(), NodeRole::Secondary);
    assert!(net.node(1).unwrap().is_consistent());
    assert!(net.node(2).unwrap().is_consistent());
}

/// Three nodes all hold independent elections. After heartbeats are
/// exchanged, only the lowest-ID node remains Primary — the others yield
/// upon receiving its heartbeat.
#[test]
fn three_node_election_lowest_id_wins() {
    let mut net = SimulatedNetwork::new();
    net.add_node(10);
    net.add_node(20);
    net.add_node(30);

    // Each node initially Unknown
    for id in &[10, 20, 30] {
        assert_eq!(net.node(*id).unwrap().role(), NodeRole::Unknown);
    }

    // All three nodes independently elect themselves Primary
    // (each ran NoOtherNodesFound before hearing from others)
    for id in &[10, 20, 30] {
        let t = net.node_mut(*id).unwrap().process_event(
            tpt_e_swarm_sync::state_machine::Event::NoOtherNodesFound,
        );
        assert_eq!(t.new_role, NodeRole::Primary);
    }

    // Now all three think they are Primary — exchange heartbeats.
    // Node 10 (lowest ID) sends heartbeats to 20 and 30.
    net.send_heartbeat(10, 20);
    net.send_heartbeat(10, 30);
    // Node 20 sends heartbeats to 10 and 30.
    net.send_heartbeat(20, 10);
    net.send_heartbeat(20, 30);
    // Node 30 sends heartbeats to 10 and 20.
    net.send_heartbeat(30, 10);
    net.send_heartbeat(30, 20);

    net.deliver_all();

    // After reconciliation: node 10 (lowest ID) remains Primary;
    // nodes 20 and 30 yield because they receive heartbeats from
    // a lower-ID node.
    assert_eq!(net.node(10).unwrap().role(), NodeRole::Primary);
    assert_eq!(net.node(20).unwrap().role(), NodeRole::Secondary);
    assert_eq!(net.node(30).unwrap().role(), NodeRole::Secondary);
    assert!(net.node(10).unwrap().is_consistent());
    assert!(net.node(20).unwrap().is_consistent());
    assert!(net.node(30).unwrap().is_consistent());
}

/// After a partition isolating the Primary, the Secondaries detect
/// partition and one eventually promotes. After heal, exactly one Primary
/// remains — the lowest-ID node across both partitions.
#[test]
fn partition_and_heal_convergence() {
    let mut net = SimulatedNetwork::new();
    net.add_node(1);
    net.add_node(2);
    net.add_node(3);

    // Node 1 becomes Primary
    let _ = net.node_mut(1).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::NoOtherNodesFound,
    );
    net.send_heartbeat(1, 2);
    net.send_heartbeat(1, 3);
    net.deliver_all();
    assert_eq!(net.node(2).unwrap().role(), NodeRole::Secondary);
    assert_eq!(net.node(3).unwrap().role(), NodeRole::Secondary);

    // Partition: isolate node 1 from nodes 2 and 3
    net.partition(&[1], &[2, 3]);
    net.deliver_all();

    // Nodes 2 and 3 detect partition, then heartbeat timeout → promote
    let _ = net.node_mut(2).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::PartitionDetected,
    );
    let _ = net.node_mut(3).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::PartitionDetected,
    );
    let _ = net.node_mut(2).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::HeartbeatTimeout,
    );
    let _ = net.node_mut(3).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::HeartbeatTimeout,
    );

    // Heal the partition
    net.heal_partition(&[1], &[2, 3]);
    // Exchange heartbeats for reconciliation
    net.send_heartbeat(1, 2);
    net.send_heartbeat(1, 3);
    net.send_heartbeat(2, 1);
    net.send_heartbeat(2, 3);
    net.send_heartbeat(3, 1);
    net.send_heartbeat(3, 2);
    net.deliver_all();

    // After healing and heartbeats, exactly one Primary remains
    let primaries: Vec<u32> = [1, 2, 3]
        .iter()
        .filter(|&&id| net.node(id).unwrap().role() == NodeRole::Primary)
        .copied()
        .collect();
    assert_eq!(primaries.len(), 1, "exactly one Primary must exist, got: {:?}", primaries);
    assert_eq!(primaries[0], 1, "lowest-ID node (1) should be the surviving Primary");

    for id in &[1, 2, 3] {
        assert!(net.node(*id).unwrap().is_consistent());
    }
}

/// A node that discovers the mesh via heartbeat starts as Secondary.
#[test]
fn node_discovery_via_heartbeat() {
    let mut net = SimulatedNetwork::new();
    net.add_node(1);
    net.add_node(2);

    // Node 1 becomes Primary
    let _ = net.node_mut(1).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::NoOtherNodesFound,
    );

    // Deliver heartbeat from node 1 to node 2
    net.send_heartbeat(1, 2);
    net.deliver_all();

    assert_eq!(net.node(2).unwrap().role(), NodeRole::Secondary);
    assert!(net.node(1).unwrap().is_consistent());
    assert!(net.node(2).unwrap().is_consistent());
}

/// A partitioned link must actually block delivery — regression test for a
/// bug where `SimulatedNetwork::partition()` set the link's `active` flag
/// but `send_heartbeat` never consulted it, so messages sent across a
/// "partitioned" link were silently delivered anyway. Every prior partition
/// test worked around this by simply not calling `send_heartbeat` across
/// the partition; this test calls it anyway and asserts it has no effect.
#[test]
fn partitioned_link_blocks_heartbeat_delivery() {
    let mut net = SimulatedNetwork::new();
    net.add_node(1);
    net.add_node(2);

    net.partition(&[1], &[2]);
    // Node 1 sends a heartbeat straight across the partitioned link.
    net.send_heartbeat(1, 2);
    net.deliver_all();

    // Node 2 must not have received it — still Unknown, not Secondary.
    assert_eq!(net.node(2).unwrap().role(), NodeRole::Unknown);

    // Healing the partition must restore delivery: the same heartbeat that
    // was silently dropped above now gets through (any heartbeat while
    // Unknown implies a Primary exists, so node 2 becomes Secondary).
    net.heal_partition(&[1], &[2]);
    net.send_heartbeat(1, 2);
    net.deliver_all();
    assert_eq!(net.node(2).unwrap().role(), NodeRole::Secondary);
}

/// A 100% drop rate on a link must behave like a partition: no message
/// gets through, however many times it's sent.
#[test]
fn full_drop_rate_blocks_all_heartbeats() {
    let mut net = SimulatedNetwork::new();
    net.add_node(1);
    net.add_node(2);

    net.set_drop_rate(1, 2, 1000); // 100.0%
    for _ in 0..20 {
        net.send_heartbeat(1, 2);
    }
    net.deliver_all();

    assert_eq!(net.node(2).unwrap().role(), NodeRole::Unknown);
}

/// A 0% drop rate (the default) must never drop a message — sanity check
/// for `link_allows_delivery`'s permille comparison direction.
#[test]
fn zero_drop_rate_never_blocks() {
    let mut net = SimulatedNetwork::new();
    net.add_node(1);
    net.add_node(2);

    net.set_drop_rate(1, 2, 0);
    let _ = net.node_mut(1).unwrap().process_event(
        tpt_e_swarm_sync::state_machine::Event::NoOtherNodesFound,
    );
    net.send_heartbeat(1, 2);
    net.deliver_all();

    assert_eq!(net.node(2).unwrap().role(), NodeRole::Secondary);
}

/// Stress test: repeated random partition/heal cycles and lossy heartbeat
/// storms across a 4-node mesh never leave more than one Primary standing,
/// and every node's local invariant holds throughout — not just at the end.
#[test]
fn randomized_partition_and_brownout_stress() {
    let ids = [1u32, 2, 3, 4];
    let mut net = SimulatedNetwork::new();
    for id in ids {
        net.add_node(id);
    }

    // Simple xorshift so this test is deterministic and reproducible.
    let mut rng: u32 = 0xC0FF_EE11;
    let mut next = |bound: u32| {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng % bound
    };

    // Everyone races for Primary at the start (worst case for divergence).
    for id in ids {
        let _ = net.node_mut(id).unwrap().process_event(
            tpt_e_swarm_sync::state_machine::Event::NoOtherNodesFound,
        );
    }

    for round in 0..200 {
        // Randomly partition or heal a random pair.
        let a = ids[next(4) as usize];
        let mut b = ids[next(4) as usize];
        while b == a {
            b = ids[next(4) as usize];
        }
        if round % 2 == 0 {
            net.partition(&[a], &[b]);
        } else {
            net.heal_partition(&[a], &[b]);
        }

        // Random lossy link somewhere in the mesh.
        let c = ids[next(4) as usize];
        let mut d = ids[next(4) as usize];
        while d == c {
            d = ids[next(4) as usize];
        }
        net.set_drop_rate(c, d, next(1001));

        // Full heartbeat exchange among all pairs.
        for &from in &ids {
            for &to in &ids {
                if from != to {
                    net.send_heartbeat(from, to);
                }
            }
        }
        net.deliver_all();

        for id in ids {
            assert!(
                net.node(id).unwrap().is_consistent(),
                "node {id} inconsistent at round {round}"
            );
        }
    }

    // Heal everything and let the mesh fully reconcile.
    for &a in &ids {
        for &b in &ids {
            if a != b {
                net.heal_partition(&[a], &[b]);
                net.set_drop_rate(a, b, 0);
            }
        }
    }
    for _ in 0..5 {
        for &from in &ids {
            for &to in &ids {
                if from != to {
                    net.send_heartbeat(from, to);
                }
            }
        }
        net.deliver_all();
    }

    let primaries: Vec<u32> = ids
        .iter()
        .filter(|&&id| net.node(id).unwrap().role() == NodeRole::Primary)
        .copied()
        .collect();
    assert!(
        primaries.len() <= 1,
        "at most one Primary must remain after full reconciliation, got: {:?}",
        primaries
    );
}

/// Node count matches what was added.
#[test]
fn simulated_network_node_count() {
    let mut net = SimulatedNetwork::new();
    assert_eq!(net.node_count(), 0);
    net.add_node(1);
    assert_eq!(net.node_count(), 1);
    net.add_node(2);
    assert_eq!(net.node_count(), 2);
}
