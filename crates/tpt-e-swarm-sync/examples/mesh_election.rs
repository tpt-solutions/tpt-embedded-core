//! Minimal example: two nodes discover each other and elect a Primary.
//!
//! Run with:
//!
//! ```text
//! cargo run -p tpt-e-swarm-sync --example mesh_election --features mock
//! ```

use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

fn main() {
    let mut node_a = MeshStateMachine::new(1);
    let mut node_b = MeshStateMachine::new(2);

    // Node A finds no other nodes yet and becomes Primary.
    let t = node_a.process_event(Event::NoOtherNodesFound);
    println!("node_a -> {:?} (broadcast: {})", t.new_role, t.should_broadcast);

    // Node B hears node A's heartbeat and becomes Secondary.
    let t = node_b.process_event(Event::HeartbeatReceived);
    println!("node_b -> {:?} (broadcast: {})", t.new_role, t.should_broadcast);

    assert_eq!(node_a.role(), NodeRole::Primary);
    assert_eq!(node_b.role(), NodeRole::Secondary);
    assert!(node_a.is_consistent());
    assert!(node_b.is_consistent());

    println!("mesh converged: node_a is Primary, node_b is Secondary");
}
