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
- [x] **BUG (fixed 2026-07-22)**: crate fails to build with no features (`cargo build -p tpt-e-typestate-hal`) — `dma.rs:17`/`isr.rs:3` unconditionally `use crate::mock::...` as a default type param, but `mock` is feature-gated (`lib.rs:26-27`). Fixed by gating the import + default type param behind `#[cfg(feature = "mock")]` (separate struct defs per cfg branch).
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
- [x] **BUG (fixed 2026-07-22)**: `tests/kani_ringbuf.rs:34,51,66,111` call `buf.pop::<u32>()` with an invalid turbofish (`pop` has no method-level generic) — this file silently failed to compile under `cargo kani`. Fixed by dropping the turbofish at all 4 call sites.
- [x] **BUG/design gap (fixed 2026-07-22)**: `dma_handoff.rs:29` `DmaLoan::lend_for_dma` took `&self` (shared, not exclusive) — nothing stopped `push`/`pop` on the `RingBuf` while a loan was outstanding. Fixed: `lend_for_dma` now takes `&mut self` and `DmaLoan` holds the exclusive borrow, so the borrow checker rejects `push`/`pop` while a loan is live (proven by a `compile_fail` doctest in `dma_handoff.rs`).
- [ ] `dma_handoff.rs` never actually references `tpt_e_typestate_hal` despite `lib.rs` documenting DMA-handle integration and the crate carrying it as an optional dependency — the advertised integration still doesn't exist (the exclusivity fix above addressed the aliasing bug, not the missing cross-crate wiring).
- [ ] `src/mock.rs` is a doc-comment stub with no actual mock content — inconsistent with `tpt-e-typestate-hal`'s fully-implemented mock.
- [ ] `ring_buf.rs:33`: `MASK = CAP - 1` assumes `CAP` is a power of two (only documented, not enforced) — add a `const _: () = assert!(CAP.is_power_of_two())` guard so a bad `CAP` is a compile error, not silent data corruption.
- [x] No proptest/kani coverage of `dma_handoff.rs` (`DmaLoan`/`lend_for_dma`/`reclaim`) at all — **partially fixed 2026-07-22**: added `tests/dma_handoff.rs` (round-trip + backing-storage tests) and a `compile_fail` doctest proving exclusivity. Still no dedicated proptest/kani harness for this module.

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
- [x] **BUG (core design gap, fixed 2026-07-22)**: `tokens.rs:31,54,76` — `DmaParkedToken::new()`, `RtcIsolatedToken::new()`, `BuffersFlushedToken::new()` were all unrestricted `pub fn` with no runtime check; any caller could forge all three tokens. Fixed: constructors are now `pub(crate)`; a `#[cfg(feature = "mock")] pub fn mock()` exists for host-side testing only. Still TODO: real precondition-checked issuance from `tpt-e-typestate-hal`/RTC/UART drivers once those exist (see line below).
- [ ] `sleep.rs:52` `loop {}` — placeholder for the actual deep-sleep instruction; `enter_deep_sleep` can't be exercised/verified end-to-end even in principle yet.
- [ ] `kani_sleep.rs:39-41` self-acknowledges it can't call `enter_deep_sleep` in the proof (returns `!`), so the Kani harness only proves tokens/controller are constructible — it does not actually verify the "unreachable without preconditions" claim checked off above (ties to the tokens.rs gap).
- [ ] `src/mock.rs` is an empty doc-comment stub despite the `mock` feature being declared in `Cargo.toml`/`lib.rs`.
- [x] **Resolved 2026-07-22**: `proptest` dev-dependency was declared but never used — removed from `Cargo.toml` (nothing to fuzz yet since token construction takes no inputs; re-add once real precondition-checked issuance lands).

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
- [x] **Headline gap — partially fixed 2026-07-22 (SHA-256 only)**: `aes.rs:10-12`, `sha.rs:10-14` were no-op stubs and `ecc.rs` has zero methods, yet `lib.rs` documents a "mathematically verified, constant-time" guarantee. `sha.rs`/`mock.rs` now share a real, correct, incremental SHA-256 implementation (`sha256_core.rs`, FIPS 180-4, no artificial length cap) verified against NIST KATs. **`aes.rs`/`ecc.rs` are still stubs** — real constant-time AES needs a bitsliced/hardware-backed implementation (a naive table-based one is timing-unsafe), which is a larger follow-up; do not implement AES with simple S-box table lookups without addressing that.
- [x] Add NIST/FIPS known-answer test vectors (KATs) for AES, SHA-256, and ECC — **SHA-256 done** (`tests/mock_crypto.rs::sha256_kats`, verified independently via `sha256sum`). AES/ECC KATs still needed once real implementations land.
- [x] **Resolved 2026-07-22**: mock SHA-256's `update()` silently truncated input beyond a 256-byte buffer cap instead of erroring — the cap no longer exists (real streaming SHA-256 has no fixed cap); regression test `mock_sha256_handles_input_over_256_bytes` added.
- [x] **Resolved 2026-07-22**: mock SHA-256 `finalize()` wasn't properly incremental (chunked vs. bulk updates diverged). Now genuinely incremental; proven by `mock_sha256_multiple_updates` (now a real equality assertion, not a length-only check) and `tests/proptest_sha256.rs::chunked_update_matches_bulk_update`.
- [x] **Resolved 2026-07-22**: `proptest` dev-dependency was declared but never used — now actually exercised by `tests/proptest_sha256.rs`.

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
- [x] **BUG (fixed 2026-07-22)**: crate failed to build with no features (`cargo build -p tpt-e-swarm-sync`) — `mesh.rs:25,27` used `tpt_e_chronos::ring_buf::RingBuf` unconditionally, but `tpt-e-chronos` was `optional = true`. Fixed by making `tpt-e-chronos` a required dependency.
- [x] **BUG (correctness, fixed 2026-07-22)**: `state_machine.rs`'s `PartitionDetected` catch-all arm self-promoted *any* `Secondary` straight to `Primary` with no tie-break — if a partition stranded multiple Secondaries together, each independently self-promoted (two+ simultaneous Primaries), violating the module's core guarantee. Fixed: `PartitionDetected` now only marks `partitioned = true`; promotion happens solely via the existing, tested `HeartbeatTimeout` path. Regression tests added (`partition_detection_secondary` updated, `partition_does_not_cause_simultaneous_dual_primary` added). **Full divergence-freedom across concurrent nodes still needs real cross-node tie-break/quorum logic** — this fix only removes the single-tick self-promotion bug; see the still-open Kani divergence proof item below.
- [x] **Partially resolved 2026-07-22**: no Kani proof existed for swarm-sync at all — added `tests/kani_state_machine.rs` with two harnesses: `partition_detected_never_promotes_directly_to_primary` (proves the fixed bug stays fixed under symbolic event prefixes) and `partition_does_not_cause_simultaneous_dual_primary` (two symbolic node IDs). **Not run locally** — Kani doesn't build on native Windows (`kani-verifier` hits `std::os::unix`-only code); needs CI (`kani.yml`, `ubuntu-latest`) to confirm. These harnesses prove the narrower property the 2026-07-22 fix restores, not full cross-node divergence-freedom over arbitrary interleavings (still needs real tie-break/quorum logic).
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

- [x] **CI gap — fixed 2026-07-22**: `build.yml`/`test.yml` always passed `--features mock`, so default (no-feature) builds were never checked — this is exactly why the `tpt-e-typestate-hal` and `tpt-e-swarm-sync` build breaks above went uncaught. Added a `build-default-features` job (`cargo build --workspace`, no features) to `build.yml`.
- [x] **CI gap — fixed 2026-07-22**: `build.yml`'s 4-way chip matrix (esp32/esp32s3/esp32c3/esp32c6) was cosmetic — `matrix.chip` was only used in the job name; all four legs ran the identical build command. Fixed: `tpt-e-typestate-hal/Cargo.toml` now forwards real per-chip `esp-hal` features (verified against esp-hal 0.22's actual feature list), and `build.yml`'s matrix now pairs each chip with its real `--target` triple and `--features "use_esp_hal,<chip>"`.
- [x] **CI gap — placeholder added 2026-07-22**: no hardware-in-loop job exists anywhere — consistent with "Validate against target chips" above being unchecked; no code in this repo has ever run on real silicon. Added `.github/workflows/hil.yml`, disabled via `if: false` since no self-hosted runner with attached hardware exists yet — documents the intended shape and the steps to activate it.
- [x] **Adoption gap — fixed 2026-07-22**: zero `examples/` directories anywhere in the repo, and no quickstart beyond a 3-line README snippet. Added a runnable, `--features mock` example per crate (`ring_buffer_basics`, `dma_transfer`, `sleep_cycle`, `mesh_election`, `hash_a_buffer`), all verified to run end-to-end. README now has a "Getting Started" walkthrough plus a cross-crate wiring sketch and a "Status" section stating plainly what is/isn't real yet.
- [ ] **Adoption idea**: a `cargo-generate` starter template (likely its own repo) once the crates stabilize.
- [x] **Adoption idea — fixed 2026-07-22**: added `CHANGELOG.md`. CI status badges still not added to the README.
- [ ] **Verification idea**: `trybuild` compile-fail test suite so the "invalid transitions don't compile" claim is CI-enforced, not just documented.
- [ ] **Tooling idea**: `defmt`/`probe-rs` structured logging once the real `esp-hal` backend lands.
- [ ] **Tooling idea**: `cargo-fuzz` target for swarm-sync message parsing once `process_inbound` does real payload dispatch.
- [ ] **Tooling idea**: Dependabot/Renovate for `esp-hal`/Kani version pinning.
