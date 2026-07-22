# tpt-e-swarm-sync

Deterministic, state-consistent coordination for ESP32 swarms.

## Overview

`tpt-e-swarm-sync` implements a formally verified state machine (derived
from a TLA+ specification) for mesh node coordination over ESP-NOW or
802.15.4. It handles message sequencing, acknowledgments, and partition
recovery.

## Key Types

- `MeshStateMachine` — Core state machine (Primary/Secondary/Unknown)
- `MeshNode` — Full mesh node with ring-buffered message queues
- `MessageType` — Heartbeat, Election, Ack, Data
- `SimulatedNetwork` — Multi-node test harness (feature = "mock")

## State Machine

```text
Unknown → Secondary (discovered mesh)
Unknown → Primary (no other node found)
Secondary → Primary (heartbeat timeout)
Primary → Secondary (higher-priority node found)
```

## Divergence Guarantee

The state machine prevents two nodes from simultaneously believing they
are Primary. Election uses deterministic tie-breaking based on node IDs.

## Example

```rust
use tpt_e_swarm_sync::state_machine::{MeshStateMachine, Event, NodeRole};

let mut sm = MeshStateMachine::new(1);
let t = sm.process_event(Event::NoOtherNodesFound);
assert_eq!(t.new_role, NodeRole::Primary);
```

## Simulated Network

```rust
use tpt_e_swarm_sync::mock::SimulatedNetwork;

let mut net = SimulatedNetwork::new();
net.add_node(1);
net.add_node(2);
net.send_heartbeat(1, 2);
net.deliver_all();
assert_eq!(net.node(2).unwrap().role(), NodeRole::Secondary);
```
