# `tpt-e-swarm-sync`

Verified state machine for mesh coordination on ESP32 swarms.

Part of [`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core),
a proof-native `no_std` foundation for ESP32 ecosystems.

## What it does

A deterministic state machine (derived from a TLA+ specification) for mesh
node coordination over ESP-NOW/802.15.4, built on `tpt-e-chronos` for
panic-free message queuing.

- `MeshStateMachine` / `MeshNode` — Primary/Secondary/Unknown roles with
  deterministic, node-ID-based tie-breaking (a Primary only yields to a
  genuinely lower-ID node, proven by a Kani harness)
- Message sequencing and ack correlation via a per-node monotonic sequence
  counter and a bounded pending-ack set
- `SimulatedNetwork` (feature = `mock`) — a host-side test harness with
  partition/brownout/drop-rate injection

## Divergence guarantee

The state machine is designed so two nodes never simultaneously believe
they are Primary — proven for single-tick promotion and tie-breaking via
Kani, and exercised under randomized partition/heal/drop-rate stress tests.
Full cross-node divergence-freedom over arbitrary interleavings (quorum
logic) is still open — see the workspace root's `todo.md`.

## Example

```rust
use tpt_e_swarm_sync::state_machine::{MeshStateMachine, Event, NodeRole};

let mut sm = MeshStateMachine::new(1);
let t = sm.process_event(Event::NoOtherNodesFound);
assert_eq!(t.new_role, NodeRole::Primary);
```

Run it: `cargo run -p tpt-e-swarm-sync --example mesh_election --features mock`

## License

Dual-licensed under MIT OR Apache-2.0. See the
[repository root](https://github.com/tpt-solutions/tpt-embedded-core) for
full docs, architecture, and the other four crates.
