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

### `tpt-e-chronos`
- [x] Design heapless ring buffer (const-generic capacity) for time-series/telemetry data
- [x] Implement atomic / critical-section-minimal push (ISR-safe) and pop (main-loop) operations
- [x] Implement zero-copy handoff path to DMA using `tpt-e-typestate-hal` safe handles
- [x] Build `mock` feature for host-side testing
- [x] Proptest: push-N/pop-N leaves buffer empty regardless of interleaving; no data loss/corruption under randomized ISR/main-loop interleavings
- [x] Kani proof: absence of panics (no out-of-bounds access) under any push/pop interleaving
- [x] Kani/analysis: prove WCET bound for `push()` and `pop()`
- [x] Document public API + safety invariants

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

---

## Phase 4 — Distributed

### `tpt-e-swarm-sync`
- [ ] Draft formal state-machine spec (TLA+ or equivalent) for mesh coordination (ESP-NOW / 802.15.4)
- [ ] Translate spec into Rust state machine implementation
- [ ] Implement message sequencing, acknowledgment, and partition-recovery logic
- [ ] Integrate `tpt-e-chronos` for deterministic, panic-free message queuing during network events
- [ ] Build `mock` feature: simulated network harness for host-side testing (partition injection, brownout simulation)
- [ ] Proptest: randomized network failure models (drops, partitions, reordering, brownouts) never desynchronize state
- [ ] Kani proof: state machine cannot reach a divergent state (e.g., two nodes both believing they are primary) under defined failure models
- [ ] Document public API + protocol/state-machine invariants
- [ ] Phase 4 exit: CI green on all target chips; integration test combining `tpt-e-chronos` + `tpt-e-swarm-sync` under simulated partition/brownout

---

## Cross-Cutting / Ongoing

- [ ] Optional nightly HIL job: flash minimal test harness to real ESP32-S3, verify no_std abstractions map correctly to hardware
- [ ] Documentation site (rustdoc hosting or mdBook) unifying all five crates' philosophy + API docs
- [ ] Evaluate Creusot as a supplementary formal verification tool (spec mentions it alongside Kani but only Kani is detailed in §7 — treat as stretch goal, not a phase blocker)
- [ ] Track crate interdependency versioning as workspace evolves (`tpt-e-typestate-hal` is foundational — breaking changes ripple to `tpt-e-chronos`, `tpt-e-cipher`, `tpt-e-slumber`, `tpt-e-swarm-sync`)
- [ ] Periodic re-validation across all target chips (ESP32, ESP32-S3, ESP32-C3/C6) as `esp-hal` upstream evolves
- [ ] Public release checklist per crate (crates.io metadata, changelog, semver policy) once independently publishable
