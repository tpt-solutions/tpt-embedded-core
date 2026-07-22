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
}

impl SimulatedNetwork {
    /// Create a new empty simulated network.
    pub fn new() -> Self {
        Self {
            nodes: heapless::Vec::new(),
            links: heapless::Vec::new(),
        }
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
    pub fn send_heartbeat(&mut self, from: u32, to: u32) {
        let msg = Message {
            sequence: from as u64,
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
