//! Simulated network harness for host-side testing.
//!
//! Provides a `SimulatedNetwork` that connects multiple `MeshNode` instances
//! and supports injecting partitions, message drops, and brownout conditions
//! — all without real hardware or radio peripherals.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::mesh::{Message, MessageType, MeshNode};

/// A simulated network link between two nodes.
///
/// Tracks whether the link is active and supports drop injection.
#[derive(Debug)]
struct Link {
    active: AtomicBool,
    drop_rate: AtomicU32, // 0–1000, representing 0.0%–100.0% (permille)
}

impl Link {
    fn new() -> Self {
        Self {
            active: AtomicBool::new(true),
            drop_rate: AtomicU32::new(0),
        }
    }
}

/// A simulated network connecting multiple mesh nodes.
///
/// Supports partition injection, message drop simulation, and brownout
/// conditions — useful for testing the mesh state machine under failure.
///
/// # Example
///
/// ```rust
/// use tpt_e_swarm_sync::mock::SimulatedNetwork;
///
/// let mut net = SimulatedNetwork::new();
/// net.add_node(1);
/// net.add_node(2);
///
/// // Both nodes start as Unknown
/// assert_eq!(net.node(1).unwrap().role(), tpt_e_swarm_sync::state_machine::NodeRole::Unknown);
///
/// // Deliver a heartbeat from node 2 to node 1
/// net.send_heartbeat(2, 1);
/// net.deliver_all();
///
/// // Node 1 should now be Secondary (discovered a Primary)
/// assert_eq!(net.node(1).unwrap().role(), tpt_e_swarm_sync::state_machine::NodeRole::Secondary);
/// ```
#[derive(Debug)]
pub struct SimulatedNetwork {
    nodes: heapless::Vec<MeshNode, 16>,
    links: heapless::Vec<(u32, u32, Link), 32>,
    /// State for the deterministic mock PRNG used to roll drop-rate checks.
    /// Not cryptographically secure — this is a test-only network simulator.
    rng_state: u32,
}

impl SimulatedNetwork {
    /// Create a new empty simulated network.
    pub fn new() -> Self {
        Self {
            nodes: heapless::Vec::new(),
            links: heapless::Vec::new(),
            rng_state: 0x9E37_79B9, // arbitrary nonzero seed
        }
    }

    /// Advance the mock PRNG and return a value in `0..1000` (permille).
    fn next_permille(&mut self) -> u32 {
        // xorshift32 — deterministic and NOT cryptographically secure;
        // sufficient for reproducible drop-rate simulation in tests.
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        x % 1000
    }

    /// Whether a message from `from` to `to` should be delivered right now:
    /// the link must be active (not partitioned) and must not be dropped by
    /// its configured `drop_rate`.
    fn link_allows_delivery(&mut self, from: u32, to: u32) -> bool {
        let mut active = true;
        let mut drop_rate = 0u32;
        for (a, b, link) in self.links.iter() {
            if *a == from && *b == to {
                active = link.active.load(Ordering::Relaxed);
                drop_rate = link.drop_rate.load(Ordering::Relaxed);
                break;
            }
        }
        if !active {
            return false;
        }
        if drop_rate == 0 {
            return true;
        }
        self.next_permille() >= drop_rate
    }

    /// Add a node with the given ID to the network.
    ///
    /// Automatically creates links to all existing nodes.
    pub fn add_node(&mut self, node_id: u32) {
        let node = MeshNode::new(node_id);
        let existing: heapless::Vec<u32, 16> =
            self.nodes.iter().map(|n| n.node_id()).collect();
        let _ = self.nodes.push(node);
        for &other_id in &existing {
            let _ = self.links.push((node_id, other_id, Link::new()));
            let _ = self.links.push((other_id, node_id, Link::new()));
        }
    }

    /// Get a reference to a node by ID.
    pub fn node(&self, node_id: u32) -> Option<&MeshNode> {
        self.nodes.iter().find(|n| n.node_id() == node_id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn node_mut(&mut self, node_id: u32) -> Option<&mut MeshNode> {
        self.nodes.iter_mut().find(|n| n.node_id() == node_id)
    }

    /// Send a heartbeat message from one node to another.
    ///
    /// Respects the link's partition state and configured drop rate: a
    /// message across a partitioned (`partition()`-deactivated) link, or one
    /// unlucky enough to roll below `set_drop_rate`'s permille threshold, is
    /// silently dropped — same as a real lossy/partitioned radio link.
    pub fn send_heartbeat(&mut self, from: u32, to: u32) {
        if !self.link_allows_delivery(from, to) {
            return;
        }
        // Real per-node monotonic sequence number, not the sender's node
        // ID: `from`'s own `MeshNode` tracks its own outbound sequence
        // counter, incremented on every heartbeat it sends.
        let Some(sequence) = self.node_mut(from).map(MeshNode::next_outbound_sequence) else {
            return;
        };
        let msg = Message {
            sender_id: from,
            sequence,
            msg_type: MessageType::Heartbeat,
            payload: [0u8; 256],
        };
        if let Some(node) = self.node_mut(to) {
            let _ = node.receive_message(msg);
        }
    }

    /// Deliver all pending messages across the network.
    pub fn deliver_all(&mut self) {
        let node_ids: heapless::Vec<u32, 16> =
            self.nodes.iter().map(|n| n.node_id()).collect();
        for &id in &node_ids {
            if let Some(node) = self.node_mut(id) {
                while let Some(event) = node.process_inbound() {
                    let _ = node.process_event(event);
                }
            }
        }
    }

    /// Inject a partition: disable links between two node groups.
    ///
    /// All links between any node in `group_a` and any node in `group_b`
    /// are deactivated.
    pub fn partition(&mut self, group_a: &[u32], group_b: &[u32]) {
        for link in self.links.iter_mut() {
            let (a, b, ref mut l) = link;
            let cross = (group_a.contains(a) && group_b.contains(b))
                || (group_b.contains(a) && group_a.contains(b));
            if cross {
                l.active.store(false, Ordering::Relaxed);
            }
        }
    }

    /// Heal a previously injected partition.
    pub fn heal_partition(&mut self, group_a: &[u32], group_b: &[u32]) {
        for link in self.links.iter_mut() {
            let (a, b, ref mut l) = link;
            let cross = (group_a.contains(a) && group_b.contains(b))
                || (group_b.contains(a) && group_a.contains(b));
            if cross {
                l.active.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Set the drop rate for a specific link (permille: 0–1000).
    pub fn set_drop_rate(&mut self, from: u32, to: u32, permille: u32) {
        for link in self.links.iter_mut() {
            let (a, b, ref mut l) = link;
            if *a == from && *b == to {
                l.drop_rate.store(permille, Ordering::Relaxed);
            }
        }
    }

    /// Returns the number of nodes in the network.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for SimulatedNetwork {
    fn default() -> Self {
        Self::new()
    }
}
