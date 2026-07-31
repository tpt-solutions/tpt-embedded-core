# Changelog — `tpt-e-chronos`

Crates in this workspace are currently version-synchronized (see the root
`README.md`'s "Versioning & Publishing" section); this per-crate log tracks
notable changes scoped to this crate specifically. See the workspace root
`CHANGELOG.md` for the cross-crate view, and `todo.md` for the full,
dated audit trail this summarizes.

## Unreleased

### Added

- `MockClock`: a manually-advancing clock for deterministic time-series
  testing, and `push_ts`/`Timestamped<T>` helpers.
- Proptest and Kani coverage for the DMA handoff path (`DmaLoan`/
  `lend_for_dma`/`reclaim`): round-trip data preservation, FIFO ordering
  across lend/reclaim cycles, and panic-freedom.
- A `const _: () = assert!(CAP.is_power_of_two())` guard so an invalid
  `RingBuf` capacity is a compile error, not silent data corruption.
- A real multi-threaded stress test (`tests/concurrency_stress.rs`)
  proving concurrent push/pop never loses or corrupts data.

### Fixed

- `RingBuf` had no actual critical section despite documenting one —
  concurrent `push`/`pop` calls could silently lose data. Fixed with a
  `critical_section::with(...)`-based critical section (not an atomic CAS
  spinlock, which doesn't compile on RISC-V targets lacking the atomic-RMW
  extension — see below).
- `DmaLoan::lend_for_dma` took a shared (`&self`) rather than exclusive
  reference, so nothing stopped `push`/`pop` while a loan was outstanding.
  Now takes `&mut self`; the borrow checker enforces exclusivity.
- Two separate instances of code that doesn't compile on
  `riscv32imc-unknown-none-elf` (no atomic-RMW extension — only `load`/
  `store` are available, not `fetch_add`/`compare_exchange`): the ring
  buffer's original CAS spinlock, and `MockClock`'s `AtomicU64` (switched
  to `core::cell::Cell<u64>`, since it's a single-threaded test utility
  with no ISR-sharing claim).
- `dma_handoff.rs`'s doctest for `transfer_with_dma` didn't actually
  compile (never caught because CI only builds with `--features mock`).
