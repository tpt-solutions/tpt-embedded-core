# Changelog

All notable changes to this workspace are documented here. Crates are
currently synchronized on a single workspace version (see `README.md` §
Versioning & Publishing); this log is workspace-wide rather than
per-crate until independent publishing begins.

## Unreleased

### Fixed

- `tpt-e-typestate-hal` and `tpt-e-swarm-sync` failed to build with no
  Cargo features enabled (`cargo build` with defaults only) — both are now
  buildable and tested without `mock`.
- `tpt-e-swarm-sync`'s state machine could let a network partition promote
  multiple stranded `Secondary` nodes to `Primary` simultaneously,
  violating its own no-dual-primary guarantee. `PartitionDetected` no
  longer unilaterally promotes; promotion happens only via the existing,
  tested heartbeat-timeout path.
- `tpt-e-chronos`'s `DmaLoan` now holds an exclusive borrow of the
  `RingBuf` it loans out, so the compiler (not just documentation)
  prevents `push`/`pop` while a DMA loan is outstanding.
- `tpt-e-slumber`'s sleep proof tokens (`DmaParkedToken`, etc.) can no
  longer be constructed from outside the crate; a `mock`-gated
  `Token::mock()` exists for host-side testing.
- `tpt-e-cipher`'s mock SHA-256 no longer silently truncates input past a
  256-byte cap and is now genuinely incremental (chunked updates match
  bulk updates) — backed by a real, NIST-KAT-verified SHA-256
  implementation shared with the (still hardware-pending) `sha::Sha256Engine`.
- `tests/kani_ringbuf.rs` had an invalid turbofish that silently prevented
  it from compiling under `cargo kani`.

### Added

- Runnable examples for every crate under `examples/` (all require
  `--features mock`).
- CI now builds the workspace with no features enabled, closing the gap
  that let the build-break fixes above go undetected.
- CI's per-chip build matrix now actually targets each chip (real
  `--target` triple + `esp-hal` chip feature) instead of running the same
  host build four times.
- A Kani harness (`tpt-e-swarm-sync/tests/kani_state_machine.rs`) guarding
  the partition/dual-primary fix above.
- A disabled placeholder hardware-in-loop workflow
  (`.github/workflows/hil.yml`) documenting what's needed to make one real.

See `todo.md` for the full, current list of known gaps and follow-up work.
