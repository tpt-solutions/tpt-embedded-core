# Changelog — `tpt-e-typestate-hal`

Crates in this workspace are currently version-synchronized (see the root
`README.md`'s "Versioning & Publishing" section); this per-crate log tracks
notable changes scoped to this crate specifically. See the workspace root
`CHANGELOG.md` for the cross-crate view, and `todo.md` for the full,
dated audit trail this summarizes.

## Unreleased

### Added

- `AesDmaChannel`: a peripheral-specific typestate wrapper (`Idle →
  Configured → Transferring → Complete`) around `esp-hal`'s real hardware
  AES-DMA, flashed and verified against the FIPS-197 known-answer vector
  on a real ESP32-C3.
- Per-chip `esp-hal` feature forwarding (`esp32`/`esp32s3`/`esp32c3`/`esp32c6`)
  so the workspace's CI build matrix can actually target each chip.
- `trybuild` compile-fail tests proving invalid typestate transitions don't
  compile, plus proptest/Kani coverage for `IsrGuard`/`MockIsrGuard`.
- Optional `defmt` structured-logging support (zero overhead when disabled).

### Fixed

- Crate failed to build with no features enabled (`mock`-only types were
  referenced unconditionally).
- `IsrOps` trait had zero real implementors due to a generic-parameter
  mismatch between the trait and both `IsrGuard` backends.

### Known limitations

- `EspHalBackend` (the generic, peripheral-agnostic DMA backend) remains
  intentionally unimplemented: no hardware peripheral on `esp32`/`esp32c3`/
  `esp32s3` performs an unmodified memory-to-memory DMA move under
  `esp-hal 0.22`, so its bare-buffer contract can't be honestly fulfilled
  on these chips. `AesDmaChannel` above is the real, working alternative.
