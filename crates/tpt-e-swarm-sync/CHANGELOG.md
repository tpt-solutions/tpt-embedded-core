# Changelog — `tpt-e-swarm-sync`

Crates in this workspace are currently version-synchronized (see the root
`README.md`'s "Versioning & Publishing" section); this per-crate log tracks
notable changes scoped to this crate specifically. See the workspace root
`CHANGELOG.md` for the cross-crate view, and `todo.md` for the full,
dated audit trail this summarizes.

## Unreleased

### Added

- Real per-message-type dispatch in `process_inbound` (`Heartbeat`,
  `Election`, `Data` with auto-ack, `Ack` with `pending_acks` correlation
  and a distinct `MessageAcknowledged` event) and a real per-node
  monotonic sequence counter (`next_outbound_sequence`), replacing the
  earlier "every message becomes `HeartbeatReceived`" placeholder.
- `cargo-fuzz` target (`fuzz/fuzz_targets/fuzz_mesh_node.rs`) fuzzing
  message parsing and the state machine's consistency invariant.
- Kani proofs for tie-breaking (`primary_only_yields_to_lower_id`) and for
  the partition/dual-primary fix below.

### Fixed

- Deterministic tie-breaking was documented but not implemented: any
  inbound `Election` message unconditionally forced the receiving Primary
  to step down, with no comparison of node IDs at all — a lower-priority
  or malicious peer could force a legitimate Primary to demote. Fixed:
  `Message` carries a `sender_id`; a Primary now yields only to a
  genuinely lower-ID node.
- A network partition could let multiple stranded `Secondary` nodes
  self-promote to `Primary` simultaneously (violating the crate's core
  no-dual-primary guarantee), and — once fixed — a further bug meant
  post-partition healing never reconciled two independently-promoted
  Primaries. Both fixed.
- `SimulatedNetwork::partition()`/`set_drop_rate()` mutated link state
  that `send_heartbeat` never actually read — partition/drop-rate
  enforcement was entirely decorative.

### Known limitations

- Full cross-node divergence-freedom over arbitrary interleavings still
  needs real quorum logic — current Kani proofs cover tie-breaking and
  the specific fixed bugs, not the general property.
- `Ack`/`Data` correlation only ties to the sending node's own
  `pending_acks` — no cross-node retransmission-on-timeout or duplicate
  detection yet.
- No target-chip CI coverage yet (host-only `mock`; no ESP32 hardware
  runner attached — see `.github/workflows/hil.yml`).
