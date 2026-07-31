# Changelog — `tpt-e-slumber`

Crates in this workspace are currently version-synchronized (see the root
`README.md`'s "Versioning & Publishing" section); this per-crate log tracks
notable changes scoped to this crate specifically. See the workspace root
`CHANGELOG.md` for the cross-crate view, and `todo.md` for the full,
dated audit trail this summarizes.

## Unreleased

### Added

- Real hardware wiring: `use_esp_hal` (plus a chip feature) wires
  `enter_deep_sleep` to the real `esp-hal` RTC deep-sleep instruction
  (`Rtc::sleep_deep`), flashed and confirmed on a real ESP32-C3 (deep
  sleep observably shut off the board's USB-Serial-JTAG peripheral as
  expected).

### Fixed

- `DmaParkedToken`/`RtcIsolatedToken`/`BuffersFlushedToken` constructors
  were unrestricted `pub fn`, so any caller could forge a proof token with
  no actual precondition check. Constructors are now `pub(crate)`, with a
  `mock`-gated `Token::mock()` for host-side testing only.
- The same three tokens derived `Copy`, which would let a token proving a
  precondition at issuance time be duplicated and reused after the
  precondition no longer holds. Dropped `Copy`/`Clone` — tokens are now
  linear (move-once).
- `MockSleepBackend::try_sleep` used `AtomicUsize::fetch_add`, which
  doesn't compile on `riscv32imc-unknown-none-elf` (no atomic-RMW
  extension) — invisible because CI only ever compiled the `mock` feature
  on host. Fixed with a non-atomic load-then-store.

### Known limitations

- Tokens only prevent *forging* preconditions from outside the crate —
  real precondition-checked issuance from `tpt-e-typestate-hal`/RTC/UART
  drivers (proving DMA is actually parked, RTC actually isolated, buffers
  actually flushed) doesn't exist yet.
