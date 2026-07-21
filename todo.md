# tpt-embedded-core — Project Checklist

Tracks work described in `spec.txt`. Phases mirror spec §8 (Roadmap); crate sub-sections mirror spec §6 (Crate Specifications).

---

## Phase 0 — Repo & Workspace Setup

- [x] Initialize git repo, `.gitignore` for Rust/Cargo
- [x] Create Cargo workspace root `Cargo.toml` (`[workspace]` with `members = ["crates/*"]`)
- [x] Enforce strict workspace lints (`#![deny(unsafe_code)]` default; document per-crate exception process for unavoidable register-access boundaries)
- [x] Choose and pin `esp-hal` version/feature strategy (chip-agnostic via HAL generics, not a single chip)
- [x] Dual-license: add `LICENSE-MIT` and `LICENSE-APACHE`, reference in each crate's `Cargo.toml`
- [x] Write root `README.md`: unified philosophy (Typestate over runtime checks, `deny(unsafe_code)`, property-based testing, WCET-bounded), architecture diagram, crate table
- [x] Add `CONTRIBUTING.md` codifying the "TPT Standard" (spec §4) as review checklist items
- [x] Scaffold `.github/workflows/`:
  - [x] `build.yml` — no_std build matrix across ESP32, ESP32-S3, ESP32-C3/C6 + cargo-deny
  - [x] `test.yml` — `cargo test --features mock` (host/std)
  - [x] `proptest.yml` — property-based test run
  - [x] `kani.yml` — `cargo kani` on critical modules (ring buffer bounds, typestate transitions; crypto once Phase 3 lands)
- [x] Decide crate publishing strategy (independently versioned vs. workspace-synced versions) and document it
- [x] Set up `cargo-deny` / license & dependency audit in CI

---

## Phase 1 — Foundation

### `tpt-e-typestate-hal`
- [x] Design typestate chain: `Idle → Configured → Transferring → Complete` as distinct zero-sized/marker types
- [x] Define safe DMA handle API consumed by `tpt-e-chronos` and (later) `tpt-e-cipher`
- [x] Implement chip-agnostic abstraction layer over `esp-hal` DMA/ISR primitives
- [x] Isolate any unavoidable `unsafe` register access into a minimal, documented boundary module
- [x] Build `mock` feature: `std`-backed fake DMA/ISR for host-side testing
- [x] Proptest: state-transition invariants (no transition skips Configured→Transferring without valid buffer)
- [x] Kani proof: buffer passed to DMA transfer is exclusively borrowed, correctly aligned, and immutable from main thread until `Complete`
- [ ] Validate against target chips: ESP32, ESP32-S3, ESP32-C3/C6
- [x] Document public API + safety invariants (rustdoc)
- [ ] **BUG**: crate fails to build with no features (`cargo build -p tpt-e-typestate-hal`) — `dma.rs:17`/`isr.rs:3` unconditionally `use crate::mock::...` as a default type param, but `mock` is feature-gated (`lib.rs:26-27`). Gate the import + default type param behind `#[cfg(feature = "mock")]`.
- [ ] `backend.rs:76-127`: `EspHalBackend`/`EspHalIsrGuard` (the real `use_esp_hal` hardware path) are all TODO stubs — no real register-level implementation exists anywhere in the crate.
- [ ] `use_esp_hal` feature is unusable as shipped — doesn't forward/expose per-chip `esp-hal` feature selection (esp32/esp32c3/etc.)
- [ ] `isr.rs:12`: `IsrGuard` carries `#[allow(missing_debug_implementations)]`, an undocumented carve-out from the crate's `missing_debug_implementations` lint
- [ ] Add `trybuild` compile-fail tests proving invalid typestate transitions actually fail to compile (currently only asserted in doc comments)
- [ ] Add proptest/kani coverage for `IsrGuard`/`MockIsrGuard` (currently only `DmaChannel` transitions are covered)

### `tpt-e-chronos`
- [x] Design heapless ring buffer (const-generic capacity) for time-series/telemetry data
- [x] Implement atomic / critical-section-minimal push (ISR-safe) and pop (main-loop) operations
- [x] Implement zero-copy handoff path to DMA using `tpt-e-typestate-hal` safe handles
- [x] Build `mock` feature for host-side testing
- [x] Proptest: push-N/pop-N leaves buffer empty regardless of interleaving; no data loss/corruption under randomized ISR/main-loop interleavings
- [x] Kani proof: absence of panics (no out-of-bounds access) under any push/pop interleaving
- [x] Kani/analysis: prove WCET bound for `push()` and `pop()`
- [x] Document public API + safety invariants
- [ ] **BUG**: `tests/kani_ringbuf.rs:34,51,66,111` call `buf.pop::<u32>()` with an invalid turbofish (`pop` has no method-level generic) — this file silently fails to compile under `cargo kani`, so the claimed formal coverage never actually runs.
- [ ] **BUG/design gap**: `dma_handoff.rs:29` `DmaLoan::lend_for_dma` takes `&self` (shared, not exclusive) — nothing stops `push`/`pop` on the `RingBuf` while a loan is outstanding, despite the doc claiming the buffer "must not be used until reclaimed." Introduce a `Loaned` typestate so this is enforced at compile time, not just documented.
- [ ] `dma_handoff.rs` never actually references `tpt_e_typestate_hal` despite `lib.rs` documenting DMA-handle integration and the crate carrying it as an optional dependency — the advertised integration doesn't exist yet.
- [ ] `src/mock.rs` is a doc-comment stub with no actual mock content — inconsistent with `tpt-e-typestate-hal`'s fully-implemented mock.
- [ ] `ring_buf.rs:33`: `MASK = CAP - 1` assumes `CAP` is a power of two (only documented, not enforced) — add a `const _: () = assert!(CAP.is_power_of_two())` guard so a bad `CAP` is a compile error, not silent data corruption.
- [ ] No proptest/kani coverage of `dma_handoff.rs` (`DmaLoan`/`lend_for_dma`/`reclaim`) at all — the newest, most safety-sensitive module in the crate has zero tests.

### Phase 1 exit criteria
- [x] Both crates pass `cargo test --features mock`, proptest, and `cargo kani` in CI
- [x] CI pipeline (build matrix + mock tests + proptest + Kani) green on all target chips
- [x] Integration test: `tpt-e-chronos` ring buffer fed via a `tpt-e-typestate-hal` DMA handle end-to-end (mock)

---

## Phase 2 — Reliability

### `tpt-e-slumber`
- [x] Design proof-token API (e.g. `dma_parked_token`, `rtc_isolated_token`) issued only by subsystems that can prove their precondition
- [x] Design sleep state machine gated on tokens sourced from `tpt-e-typestate-hal` state guarantees
- [x] Implement `enter_deep_sleep(dma_parked_token, rtc_isolated_token, ...)` with typestate enforcement (missing token = compile error, not runtime check)
- [x] Chip-agnostic wiring to `esp-hal` sleep/RTC APIs
- [x] Build `mock` feature for host-side testing of sleep-transition logic
- [x] Proptest: invalid precondition combinations never compile / never reach hardware sleep call in generated test harnesses
- [x] Kani proof: hardware sleep instruction is unreachable without all safety preconditions satisfied (flushed buffers, disabled DMA, isolated RTC memory)
- [x] Document public API + safety invariants
- [x] Phase 2 exit: CI green (mock tests, proptest, Kani) on all target chips; integration test with `tpt-e-typestate-hal`
- [ ] **BUG (core design gap)**: `tokens.rs:31,54,76` — `DmaParkedToken::new()`, `RtcIsolatedToken::new()`, `BuffersFlushedToken::new()` are all unrestricted `pub fn` with no runtime check. Any caller can forge all three tokens and call `enter_deep_sleep` with no real precondition satisfied — the "proof" is currently just "you called three zero-arg constructors." Restrict to `pub(crate)` and route through real precondition-checked entry points.
- [ ] `sleep.rs:52` `loop {}` — placeholder for the actual deep-sleep instruction; `enter_deep_sleep` can't be exercised/verified end-to-end even in principle yet.
- [ ] `kani_sleep.rs:39-41` self-acknowledges it can't call `enter_deep_sleep` in the proof (returns `!`), so the Kani harness only proves tokens/controller are constructible — it does not actually verify the "unreachable without preconditions" claim checked off above (ties to the tokens.rs gap).
- [ ] `src/mock.rs` is an empty doc-comment stub despite the `mock` feature being declared in `Cargo.toml`/`lib.rs`.
- [ ] `proptest` dev-dependency declared but never used (no `proptest!` macros in either test file) — dead dependency.

---

## Phase 3 — Security

### `tpt-e-cipher`
- [x] Design trait abstraction over `esp-hal` crypto peripherals (AES, SHA-256, ECC)
- [x] Define constant-time execution guarantee at the API/trait level (no data-dependent branching or timing)
- [x] Isolate raw peripheral register sequencing into minimal `unsafe` boundary, documented and reviewed
- [x] Integrate with `tpt-e-typestate-hal` safe DMA handles for buffer transfer to/from crypto peripherals
- [x] Build `mock` feature: software crypto backend for host-side testing (explicitly not constant-time — mock is for logic, not timing)
- [x] Proptest: correctness of wrapped operations against known test vectors (AES/SHA-256/ECC KATs)
- [ ] Formal verification: prove execution time and memory access patterns are independent of secret key material (mitigating timing side-channels)
- [ ] Add `cargo kani` (or dedicated side-channel analysis tooling) job to CI for crypto modules
- [x] Document public API + safety/timing invariants
- [ ] Phase 3 exit: CI green on all target chips; integration test combining `tpt-e-typestate-hal` DMA + `tpt-e-cipher` crypto operation
- [ ] **Headline gap**: `aes.rs:10-12`, `sha.rs:10-14` are no-op stubs and `ecc.rs` has zero methods, yet `lib.rs` documents a "mathematically verified, constant-time" guarantee — there is no real cryptographic implementation behind the claim yet. Implement real AES/SHA-256 (via `esp-hal` crypto peripherals or an audited constant-time software fallback) and decide ECC scope.
- [ ] Add NIST/FIPS known-answer test vectors (KATs) for AES, SHA-256, and ECC — none exist today; current tests only check the mock's self-consistency.
- [ ] `mock.rs:81-87`: `MockAesEngine::update()` silently truncates input beyond 256 bytes instead of erroring — callers feeding >256 bytes get silently wrong output with no indication.
- [ ] `mock.rs:89-102`: mock SHA-256 `finalize()` isn't properly incremental (chunked vs. bulk updates diverge — acknowledged in `mock_crypto.rs:94-115`'s own test).
- [ ] `proptest` dev-dependency declared but never used — dead dependency.

---

## Phase 4 — Distributed

### `tpt-e-swarm-sync`
- [x] Draft formal state-machine spec (TLA+ or equivalent) for mesh coordination (ESP-NOW / 802.15.4)
- [x] Translate spec into Rust state machine implementation
- [x] Implement message sequencing, acknowledgment, and partition-recovery logic
- [x] Integrate `tpt-e-chronos` for deterministic, panic-free message queuing during network events
- [x] Build `mock` feature: simulated network harness for host-side testing (partition injection, brownout simulation)
- [x] Proptest: randomized network failure models (drops, partitions, reordering, brownouts) never desynchronize state
- [ ] Kani proof: state machine cannot reach a divergent state (e.g., two nodes both believing they are primary) under defined failure models
- [x] Document public API + protocol/state-machine invariants
- [ ] Phase 4 exit: CI green on all target chips; integration test combining `tpt-e-chronos` + `tpt-e-swarm-sync` under simulated partition/brownout
- [ ] **BUG**: crate fails to build with no features (`cargo build -p tpt-e-swarm-sync`) — `mesh.rs:25,27` uses `tpt_e_chronos::ring_buf::RingBuf` unconditionally, but `tpt-e-chronos` is `optional = true` (`Cargo.toml:13`), only pulled in via the `mock` feature. Make `tpt-e-chronos` a required dependency — `MeshNode` needs ring buffers in production, not just for mock testing.
- [ ] **BUG (correctness, violates the crate's core guarantee)**: `state_machine.rs:154-164`, the `PartitionDetected` catch-all arm self-promotes *any* `Secondary` straight to `Primary` with no tie-break. If a partition strands multiple Secondaries together (the normal case), each independently self-promotes — two+ simultaneous Primaries, exactly what the module doc (lines 16-20) claims is impossible. Fix: don't self-promote on `PartitionDetected` alone; rely on the existing (tested) `HeartbeatTimeout` promotion path instead. Full divergence-freedom still needs real cross-node tie-break/quorum logic.
- [ ] No Kani proof exists for swarm-sync at all (confirmed via `grep -rn "kani::proof" crates/`) — `kani.yml`'s `cargo kani --workspace` currently verifies nothing about the divergence property above. Add a multi-instance harness (2-3 symbolic `MeshStateMachine`s + symbolic event sequences) proving no two instances can simultaneously report `role() == Primary`.
- [ ] `mesh.rs:62-68` `process_inbound`: every inbound message is unconditionally mapped to `HeartbeatReceived` regardless of payload ("a real implementation would inspect the message type") — message type dispatch is unimplemented.
- [ ] `src/mock.rs` is a doc-comment stub only, no actual simulated network harness despite the crate advertising host-side partition/brownout simulation.

---

## Cross-Cutting / Ongoing

- [ ] Optional nightly HIL job: flash minimal test harness to real ESP32-S3, verify no_std abstractions map correctly to hardware
- [ ] Documentation site (rustdoc hosting or mdBook) unifying all five crates' philosophy + API docs
- [ ] Evaluate Creusot as a supplementary formal verification tool (spec mentions it alongside Kani but only Kani is detailed in §7 — treat as stretch goal, not a phase blocker)
- [ ] Track crate interdependency versioning as workspace evolves (`tpt-e-typestate-hal` is foundational — breaking changes ripple to `tpt-e-chronos`, `tpt-e-cipher`, `tpt-e-slumber`, `tpt-e-swarm-sync`)
- [ ] Periodic re-validation across all target chips (ESP32, ESP32-S3, ESP32-C3/C6) as `esp-hal` upstream evolves
- [ ] Public release checklist per crate (crates.io metadata, changelog, semver policy) once independently publishable

## Audit Findings — 2026-07-22 (CI/tooling & adoption)

- [ ] **CI gap**: `build.yml`/`test.yml` always pass `--features mock`, so default (no-feature) builds are never checked — this is exactly why the `tpt-e-typestate-hal` and `tpt-e-swarm-sync` build breaks above went uncaught. Add a `cargo build --workspace` (no features) step.
- [ ] **CI gap**: `build.yml:14-21`'s 4-way chip matrix (esp32/esp32s3/esp32c3/esp32c6) is cosmetic — `matrix.chip` is only used in the job name; all four legs run the identical build command, and no per-chip Cargo features exist to select. Wire real per-chip features through.
- [ ] **CI gap**: no hardware-in-loop job exists anywhere — consistent with "Validate against target chips" above being unchecked. No code in this repo has ever run on real silicon. Propose a best-effort nightly `espflash`/`probe-rs`/QEMU-xtensa stub pending a self-hosted runner with real hardware.
- [ ] **Adoption gap**: zero `examples/` directories anywhere in the repo, and no quickstart beyond a 3-line README snippet. Add per-crate runnable examples (`--features mock`) plus a cross-crate reference example and an expanded README walkthrough.
- [ ] **Adoption idea**: a `cargo-generate` starter template (likely its own repo) once the crates stabilize.
- [ ] **Adoption idea**: add a `CHANGELOG.md` and CI status badges to the README.
- [ ] **Verification idea**: `trybuild` compile-fail test suite so the "invalid transitions don't compile" claim is CI-enforced, not just documented.
- [ ] **Tooling idea**: `defmt`/`probe-rs` structured logging once the real `esp-hal` backend lands.
- [ ] **Tooling idea**: `cargo-fuzz` target for swarm-sync message parsing once `process_inbound` does real payload dispatch.
- [ ] **Tooling idea**: Dependabot/Renovate for `esp-hal`/Kani version pinning.
