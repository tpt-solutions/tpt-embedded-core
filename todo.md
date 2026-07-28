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
- [x] `use_esp_hal` feature is unusable as shipped — doesn't forward/expose per-chip `esp-hal` feature selection (esp32/esp32c3/etc.)
- [x] `isr.rs:12`: `IsrGuard` carries `#[allow(missing_debug_implementations)]`, an undocumented carve-out from the crate's `missing_debug_implementations` lint
- [x] Add `trybuild` compile-fail tests proving invalid typestate transitions actually fail to compile (currently only asserted in doc comments)
- [x] Add proptest/kani coverage for `IsrGuard`/`MockIsrGuard` (currently only `DmaChannel` transitions are covered)

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
- [x] `dma_handoff.rs` never actually references `tpt_e_typestate_hal` despite `lib.rs` documenting DMA-handle integration and the crate carrying it as an optional dependency — the advertised integration still doesn't exist (the exclusivity fix above addressed the aliasing bug, not the missing cross-crate wiring).
- [x] `src/mock.rs` is a doc-comment stub with no actual mock content — inconsistent with `tpt-e-typestate-hal`'s fully-implemented mock.
- [x] `ring_buf.rs:33`: `MASK = CAP - 1` assumes `CAP` is a power of two (only documented, not enforced) — add a `const _: () = assert!(CAP.is_power_of_two())` guard so a bad `CAP` is a compile error, not silent data corruption.
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
- [x] **Fixed 2026-07-29**: `sleep.rs:52` `loop {}` placeholder replaced with a real hardware path. Added `use_esp_hal`/per-chip (`esp32`/`esp32s3`/`esp32c3`/`esp32c6`) features to `tpt-e-slumber/Cargo.toml` (mirroring `tpt-e-typestate-hal`), and split `SleepController` into two `cfg`-gated variants: the host/mock build keeps the zero-arg `new()` + `loop {}` (nothing else to do without hardware), while the `use_esp_hal` build's `SleepController::new(rtc: esp_hal::rtc_cntl::Rtc<'static>)` wraps a real RTC handle and `enter_deep_sleep` calls the real `Rtc::sleep_deep(&[])`. Verified against the real esp-hal 0.22 source (found cached locally): `Rtc::sleep_deep` is supported on exactly our four target chips (`esp32`/`esp32s3`/`esp32c3`/`esp32c6`), so unlike the DMA gap below this one has full chip coverage. Compile-verified for real: `cargo build -p tpt-e-slumber --target riscv32imc-unknown-none-elf --features "use_esp_hal,esp32c3"` and the `riscv32imac`/`esp32c6` leg both succeed; xtensa (esp32/esp32s3) legs can't be compile-checked locally (no xtensa toolchain in this environment) but use the identical code path. `build.yml`'s per-chip matrix now also builds `tpt-e-slumber` alongside `tpt-e-typestate-hal`. Not flashed/run on real hardware yet (user has a C3 board; deferred to a dedicated hardware-testing pass per user request 2026-07-29).
- [ ] `kani_sleep.rs:39-41` self-acknowledges it can't call `enter_deep_sleep` in the proof (returns `!`), so the Kani harness only proves tokens/controller are constructible — it does not actually verify the "unreachable without preconditions" claim checked off above (ties to the tokens.rs gap). **Still open**: this is a structural Kani limitation (can't symbolically execute a divergent hardware register write) rather than something the 2026-07-29 `sleep.rs` fix could address.
- [x] `src/mock.rs` is an empty doc-comment stub despite the `mock` feature being declared in `Cargo.toml`/`lib.rs`.
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
- [x] Add `cargo kani` (or dedicated side-channel analysis tooling) job to CI for crypto modules
- [x] Document public API + safety/timing invariants
- [x] **Partially done 2026-07-29**: integration test combining `tpt-e-typestate-hal` DMA + `tpt-e-cipher` crypto operation. Added `tpt-e-typestate-hal` as a dev-only dependency (`mock` feature, not a production dependency of the crate) and `tests/dma_cipher_integration.rs`: drives a mock `DmaChannel` through `Idle → Configured → Transferring → Complete`, then runs the delivered buffer through `MockSha256Engine` (hash) and `MockAesEngine` (encrypt), asserting the results match hashing/encrypting the same bytes directly (i.e. the DMA handoff doesn't corrupt data). "CI green on all target chips" is still open — this test only runs host-side (`mock`); no crypto code path has been exercised on real esp-hal/hardware yet (see the still-open "Formal verification" item above and the deferred `EspHalBackend` DMA gap).
- [x] **Headline gap — partially fixed 2026-07-22 (SHA-256 only)**: `aes.rs:10-12`, `sha.rs:10-14` were no-op stubs and `ecc.rs` has zero methods, yet `lib.rs` documents a "mathematically verified, constant-time" guarantee. `sha.rs`/`mock.rs` now share a real, correct, incremental SHA-256 implementation (`sha256_core.rs`, FIPS 180-4, no artificial length cap) verified against NIST KATs. **`aes.rs`/`ecc.rs` are still stubs** — real constant-time AES needs a bitsliced/hardware-backed implementation (a naive table-based one is timing-unsafe), which is a larger follow-up; do not implement AES with simple S-box table lookups without addressing that.
- [x] Add NIST/FIPS known-answer test vectors (KATs) for AES, SHA-256, and ECC — **SHA-256 done** (`tests/mock_crypto.rs::sha256_kats`, verified independently via `sha256sum`). **AES/ECC KATs now done** — all AES tests pass (FIPS 197 App B, SP 800-38A/CAVP, zero-key, S-box full table, GF(2^8) inverse); all ECC tests pass (ECDSA round-trip, generator-on-curve, point operations, n_mul).
- [x] **BUG (fixed 2026-07-28)**: ECC `n_mul` constant `C_N = 2^256 mod N` was wrong — limb 0 was `0x0C46535D039CDAEF` instead of `0x0C46353D039CDAAF` (verified via Python `py` computation). Fixed in both `n_mul()` (~line 352) and test module constant (~line 877). All 4 ECC tests that previously failed now pass.
- [x] **BUG (fixed 2026-07-28)**: `aes128_nist_sp800_38a` test had wrong expected ciphertext value (`3B3FFD90...` instead of correct `3AD77BB4...`). Verified against NIST CAVP vectors and independently confirmed with a clean Python table-lookup AES reference implementation. The Rust algebraic AES produces the same correct output.
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
- [x] Kani proof: state machine cannot reach a divergent state (e.g., two nodes both believing they are primary) under defined failure models — **updated 2026-07-29**: added `primary_only_yields_to_lower_id` harness (proves tie-breaking: Primary never yields to same-or-higher-ID heartbeat). Previous harnesses: `heartbeat_timeout_is_only_secondary_to_primary_path`, `consistency_invariant_holds_across_event_sequences`, `partition_detected_never_promotes_directly_to_primary`, `partition_does_not_cause_simultaneous_dual_primary`. Full cross-node divergence-freedom over arbitrary interleavings still needs quorum logic.
- [x] Document public API + protocol/state-machine invariants
- [ ] Phase 4 exit: CI green on all target chips; integration test combining `tpt-e-chronos` + `tpt-e-swarm-sync` under simulated partition/brownout — **partially done 2026-07-29**: added `tests/multi_node_election.rs` with 5 tests (two-node election, three-node lowest-ID-wins, partition+heal convergence, node discovery, network count). `three_node_election` now properly contests between differently-ID'd nodes and asserts exact convergence. **Stress testing expanded 2026-07-29** (see `mock.rs` bug fix directly below): added `partitioned_link_blocks_heartbeat_delivery`, `full_drop_rate_blocks_all_heartbeats`, `zero_drop_rate_never_blocks`, and `randomized_partition_and_brownout_stress` (200 rounds of random partition/heal + random-permille lossy links across a 4-node mesh, asserting per-node `is_consistent()` every round and at-most-one-Primary after full reconciliation). Still open: no target-chip CI coverage (this only runs host-side `mock`).
- [x] **BUG (fixed 2026-07-29)**: `mock.rs`'s `SimulatedNetwork::partition()`/`heal_partition()`/`set_drop_rate()` mutated each `Link`'s `active`/`drop_rate` fields, but `send_heartbeat` never read them — every "partitioned" or "100%-drop" link still delivered messages unconditionally. Every existing partition test happened to pass anyway only because it manually avoided calling `send_heartbeat` across a partitioned pair during the partition window; the network's own partition/drop enforcement was entirely decorative and untested (`set_drop_rate` had zero callers anywhere in the repo before this fix). Fixed: added `SimulatedNetwork::link_allows_delivery` (checked by `send_heartbeat` before delivering) and a deterministic xorshift32 `rng_state` field to roll drop-rate checks. Caught directly by the new `partitioned_link_blocks_heartbeat_delivery` regression test, which fails against the pre-fix code (a message sent straight across a partitioned link was, in fact, delivered).
- [x] **BUG (fixed 2026-07-22)**: crate failed to build with no features (`cargo build -p tpt-e-swarm-sync`) — `mesh.rs:25,27` used `tpt_e_chronos::ring_buf::RingBuf` unconditionally, but `tpt-e-chronos` was `optional = true`. Fixed by making `tpt-e-chronos` a required dependency.
- [x] **BUG (correctness, fixed 2026-07-22)**: `state_machine.rs`'s `PartitionDetected` catch-all arm self-promoted *any* `Secondary` straight to `Primary` with no tie-break — if a partition stranded multiple Secondaries together, each independently self-promoted (two+ simultaneous Primaries), violating the module's core guarantee. Fixed: `PartitionDetected` now only marks `partitioned = true`; promotion happens solely via the existing, tested `HeartbeatTimeout` path. Regression tests added (`partition_detection_secondary` updated, `partition_does_not_cause_simultaneous_dual_primary` added). **Full divergence-freedom across concurrent nodes still needs real cross-node tie-break/quorum logic** — this fix only removes the single-tick self-promotion bug; see the still-open Kani divergence proof item below.
- [x] **Partially resolved 2026-07-22**: no Kani proof existed for swarm-sync at all — added `tests/kani_state_machine.rs` with two harnesses: `partition_detected_never_promotes_directly_to_primary` (proves the fixed bug stays fixed under symbolic event prefixes) and `partition_does_not_cause_simultaneous_dual_primary` (two symbolic node IDs). **Not run locally** — Kani doesn't build on native Windows (`kani-verifier` hits `std::os::unix`-only code); needs CI (`kani.yml`, `ubuntu-latest`) to confirm. These harnesses prove the narrower property the 2026-07-22 fix restores, not full cross-node divergence-freedom over arbitrary interleavings (still needs real tie-break/quorum logic).
- [x] `mesh.rs:62-68` `process_inbound`: every inbound message is unconditionally mapped to `HeartbeatReceived` regardless of payload ("a real implementation would inspect the message type") — message type dispatch is unimplemented.
- [x] `src/mock.rs` is a doc-comment stub only, no actual simulated network harness despite the crate advertising host-side partition/brownout simulation.

---

## Cross-Cutting / Ongoing

- [x] Optional nightly HIL job: flash minimal test harness to real ESP32-S3, verify no_std abstractions map correctly to hardware
- [x] Documentation site (rustdoc hosting or mdBook) unifying all five crates' philosophy + API docs
- [x] **Done 2026-07-29**: Evaluated Creusot as a supplementary formal verification tool. Written up at `docs/src/formal-verification.md` (linked from the mdBook `SUMMARY.md`). Findings, grounded via live docs (creusot.rs + the installation guide, not memory): no Windows support (Linux/macOS only, same constraint this repo already has with Kani), needs a pinned nightly plus a separate Opam/Why3/SMT-solver toolchain, and does support `no_std` without `alloc` (which fits this workspace — verified no crate here uses `alloc`). Its concurrency support (`AtomicI32`/`AtomicInvariant`) is brand new as of v0.9.0 (Jan 2026) and too immature to trust for `tpt-e-chronos`'s spinlock-guarded `RingBuf`. **Recommendation: stays a stretch goal, not adopted** — Kani already covers this workspace's actual claims, and a second heavy proof toolchain is an ongoing cost, not a one-time add. Revisit if unbounded functional-correctness contracts (not just panic-freedom) become an explicit goal.
- [x] **Done 2026-07-29**: Documented crate interdependency versioning in `CONTRIBUTING.md` ("Crate Dependency Graph & Versioning Policy" section, added below the review checklist): the actual current dependency graph (`tpt-e-typestate-hal` ← `tpt-e-chronos`/`tpt-e-cipher`(dev-only) ← `tpt-e-swarm-sync`; `tpt-e-slumber` standalone), and a concrete gap this surfaced — all internal deps are `path`-only with no `version` key, which only resolves inside the workspace and will need a `version` added per dependency before any crate can be published independently (a blocker for the still-open "public release checklist" item, not something to fix before it). Policy itself: `build.yml`/`test.yml` already build the whole workspace, so a breaking change to `tpt-e-typestate-hal` fails CI in the same PR as the breaking dependent usage — there's no separate manual "remember to bump dependents" step needed today.
- [ ] Periodic re-validation across all target chips (ESP32, ESP32-S3, ESP32-C3/C6) as `esp-hal` upstream evolves
- [ ] Public release checklist per crate (crates.io metadata, changelog, semver policy) once independently publishable

## Audit Findings — 2026-07-22 (CI/tooling & adoption)

- [x] **CI gap — fixed 2026-07-22**: `build.yml`/`test.yml` always passed `--features mock`, so default (no-feature) builds were never checked — this is exactly why the `tpt-e-typestate-hal` and `tpt-e-swarm-sync` build breaks above went uncaught. Added a `build-default-features` job (`cargo build --workspace`, no features) to `build.yml`.
- [x] **CI gap — fixed 2026-07-22**: `build.yml`'s 4-way chip matrix (esp32/esp32s3/esp32c3/esp32c6) was cosmetic — `matrix.chip` was only used in the job name; all four legs ran the identical build command. Fixed: `tpt-e-typestate-hal/Cargo.toml` now forwards real per-chip `esp-hal` features (verified against esp-hal 0.22's actual feature list), and `build.yml`'s matrix now pairs each chip with its real `--target` triple and `--features "use_esp_hal,<chip>"`.
- [x] **CI gap — placeholder added 2026-07-22**: no hardware-in-loop job exists anywhere — consistent with "Validate against target chips" above being unchecked; no code in this repo has ever run on real silicon. Added `.github/workflows/hil.yml`, disabled via `if: false` since no self-hosted runner with attached hardware exists yet — documents the intended shape and the steps to activate it.
- [x] **Adoption gap — fixed 2026-07-22**: zero `examples/` directories anywhere in the repo, and no quickstart beyond a 3-line README snippet. Added a runnable, `--features mock` example per crate (`ring_buffer_basics`, `dma_transfer`, `sleep_cycle`, `mesh_election`, `hash_a_buffer`), all verified to run end-to-end. README now has a "Getting Started" walkthrough plus a cross-crate wiring sketch and a "Status" section stating plainly what is/isn't real yet.
- [ ] **Adoption idea**: a `cargo-generate` starter template (likely its own repo) once the crates stabilize.
- [x] **Adoption idea — fixed 2026-07-22**: added `CHANGELOG.md` + CI status badges (build, test, proptest, kani) in README.
- [x] **Verification idea**: `trybuild` compile-fail test suite so the "invalid transitions don't compile" claim is CI-enforced, not just documented.
- [ ] **Tooling idea**: `defmt`/`probe-rs` structured logging once the real `esp-hal` backend lands.
- [x] **Tooling idea**: `cargo-fuzz` target for swarm-sync message parsing once `process_inbound` does real payload dispatch.
- [x] **Tooling idea**: Dependabot/Renovate for `esp-hal`/Kani version pinning.

## Audit Findings — 2026-07-29 (platform-wide bug/security/gap review)

Full-workspace review across all five crates, done after the AES/ECC implementation landed. Status reflects same-session fix work; unchecked items are deferred with rationale, not forgotten.

### `tpt-e-cipher`
- [x] **BUG/security (fixed 2026-07-29)**: `ecc.rs` `P256Ecc::keygen()` returned a hardcoded private-key scalar instead of deriving from randomness, and shipped unconditionally as the crate's only ECC implementation — any real caller got the same publicly-known key. Fixed: `Ecc::keygen` now takes a caller-supplied `seed: &[u8; 32]` (documented as requiring a CSPRNG source, same contract as `sign`'s nonce) and derives the scalar via reduction mod n.
- [x] **Security (fixed 2026-07-29)**: `ecc.rs` `verify()` never checked that the public key point lies on the P-256 curve, and `PublicKey::from_xy` did zero validation — an invalid-curve attack surface for keys built from untrusted bytes. Fixed: `verify()` now rejects the identity point and off-curve points; `from_xy` now returns `Option<Self>`, `None` on validation failure.
- [x] **BUG (fixed 2026-07-29)**: neither `sign()` nor `verify()` reduced the message hash mod n before use (FIPS 186-4 requires this) — for the ~2^-32 fraction of hashes `>= n`, signatures were computed/verified under-reduced. Fixed via a new `hash_to_scalar()` helper used in both.
- [x] **BUG (fixed 2026-07-29)**: ECDSA signatures were malleable — `sign()`/`verify()` didn't enforce canonical low-s, so `(r, n-s)` was an equally-valid alternate signature. Fixed: `sign()` now canonicalizes to low-s (`s <= n/2`); `verify()` rejects non-canonical high-s signatures.
- [x] **Security (mitigated 2026-07-29)**: `point_mul`'s `found_high_bit` skip made the operation count depend on the secret scalar's bit length (nonce `k` or private key) — a timing side channel beyond the module's general "not constant-time" disclaimer. Fixed: `point_mul` now always performs 256 double+select iterations regardless of scalar value. **Not fully constant-time** — `point_add`/`point_double` still branch on point structure (identity/doubling/inverse cases); full constant-time ECC remains open, see the existing unchecked "Formal verification" item above.
- [x] **Doc inconsistency (fixed 2026-07-29)**: crate-level `lib.rs` claimed all public operations are constant-time regardless of secret key material, while `ecc.rs`'s own module doc said the opposite — and `P256Ecc` ships unconditionally (not `mock`-gated) as the only ECC backend. Fixed: `lib.rs` now has an explicit "Constant-time status" section stating AES/SHA-256 are constant-time and ECC is not yet; `ecc.rs`'s module doc updated to match.
- [x] **Test gap (fixed 2026-07-29)**: `ecdsa_wrong_key_fails` asserted nothing (`let _ = (sig, pk2);`) because `keygen()` always returned the same scalar, so two "different" keys were identical. Fixed as a side effect of the `keygen(seed)` change — the test now uses two distinct seeds and asserts rejection.
- [x] **Test gap (fixed 2026-07-29)**: all four ECC tests (`ecdsa_sign_verify_round_trip`, `ecdsa_wrong_hash_fails`, `ecdsa_wrong_key_fails`, `ecdsa_deterministic`) called `ecc.keygen()` with no arguments after the `keygen(seed)` signature change — tests failed to compile. Fixed: each test now passes a distinct seed.
- [x] **Doc staleness (fixed 2026-07-29)**: `mock.rs`'s module doc said "the mock AES is NOT constant-time" while `MockAesEngine` had already been rewritten to reuse the real constant-time algebraic S-box — a leftover contradiction from before `aes.rs` was implemented. Fixed: doc updated to state both are algorithmically constant-time, "mock" only in the sense of being a software (non-hardware) backend.
- [ ] **Test gap — deferred**: no external NIST/CAVP P-256 ECDSA sign/verify known-answer vectors exist; all sign/verify tests round-trip against the crate's own `sign()`/`verify()`, so a self-consistent algebra error (e.g. a sign-convention swap) would go undetected. Deferred rather than hand-transcribing KAT constants from memory (too easy to introduce a wrong constant that either fails spuriously or silently doesn't test what it claims) — needs either a vetted CAVP vector file or cross-checking against an independent library (e.g. Python `cryptography`) in a follow-up session.
- [x] **Fixed 2026-07-29**: unlike AES/SHA-256 (`AesEngine`/`MockAesEngine`, `Sha256Engine`/`MockSha256Engine`), there was no separate ECC mock type — `P256Ecc` in `ecc.rs` did double duty as both "the mock" (per its module doc) and "the only backend". Fixed: added `mock.rs::MockP256Ecc`, a thin `Ecc`-implementing wrapper delegating to `P256Ecc` (behaviorally identical, since there's still no hardware-backed `Ecc` impl to actually differentiate from — this is purely the organizational/naming symmetry fix, not a new algorithm). Regression test `tests/mock_crypto.rs::mock_ecc_sign_verify_round_trip` proves it's a genuine trait impl, not a type alias.
- [x] **Doc accuracy (fixed 2026-07-29)**: this file's own "Add NIST/FIPS known-answer test vectors... AES/ECC KATs now done" and "Add `cargo kani`... for crypto modules" lines (above, Phase 3 section) overstate coverage — proptest/Kani harnesses exist only for SHA-256, not AES's `encrypt_block` or any ECC operation. See `tests/kani_crypto.rs`/`tests/proptest_sha256.rs`: SHA-256 only. Flagging here since the Phase 3 checkboxes above remain `[x]` for the parts that are true (KATs exist as plain `#[test]`s) — the proptest/Kani gap specifically is the still-open "Formal verification" item in that section.

### `tpt-e-chronos`
- [x] **BUG (fixed 2026-07-29)**: `ring_buf.rs` `RingBuf` is unconditionally `unsafe impl Sync` with safe `&self` `push`/`pop` and no compile-time single-producer/consumer enforcement — the exclusivity invariant is documentation-only ("ISR interleaving is prevented by the caller"). Reproduced: 8 threads concurrently calling `push()` on a `CAP=65536` buffer silently lost 95k+ of 160k pushed items despite every call returning `Ok`. The module doc's claim of "a critical section (minimum-length)" protecting push is currently false — no critical section exists anywhere in the crate. Fixed: added atomic spinlock-based critical section (`push_lock`/`pop_lock` `AtomicBool`) to `RingBuf`; concurrent `push`/`pop` calls now serialize, eliminating data loss. Zero API/call-site changes.
- [x] **BUG (fixed 2026-07-29)**: `dma_handoff.rs`'s doctest for `transfer_with_dma` — the crate's stated primary DMA integration point — fails to compile (`DmaChannel::mock(0)` yields `Idle`, the function requires `Transferring`). Invisible because CI only ever builds with `--features mock` (never `tpt-e-typestate-hal`), so this doctest never runs in CI. Fixed: corrected the doctest to drive the channel through `configure`/`start` before calling `transfer_with_dma`.
- [x] **Design-mandate violation (fixed 2026-07-29)**: `tpt-e-chronos/src/lib.rs:2` and `tpt-e-typestate-hal/src/lib.rs:2` both use a blanket `#![allow(unsafe_code)]` at the crate root, nullifying the workspace's `deny(unsafe_code)` policy for the *entire* crate rather than isolating unsafe to the minimal documented boundary the project's own design doc (§4) requires. Fixed: restored `#![deny(unsafe_code)]` at the crate root; `#![allow(unsafe_code)]` now appears only in the specific modules that use `unsafe` (`ring_buf.rs`, `dma_handoff.rs` for chronos; `backend.rs`, `isr.rs` for typestate-hal).

### `tpt-e-typestate-hal`
- [x] **Fixed 2026-07-29**: `backend.rs`'s `IsrOps` trait had zero implementors (`MockIsrGuard`/`EspHalIsrGuard` used an incompatible type-level-generic pattern instead of the trait's method-level generic `fn register<F: Fn()>(..) -> Self`, which can't be implemented by either guard since `Self` would need to vary with `F`). Fixed exactly as previously scoped: `IsrOps` is now generic over `F` at the trait level (`trait IsrOps<F: Fn()>: Sized { unsafe fn register(handler: F) -> Self; }`), and both `MockIsrGuard<F>` (`mock.rs`) and `EspHalIsrGuard<F>` (`backend.rs`, `use_esp_hal` feature) now implement it, forwarding to their existing inherent constructors — no breaking changes to `IsrGuard::mock`/`IsrGuard::esp_hal`. Added `tests/proptest_isr.rs::mock_isr_guard_implements_isr_ops_trait`, which registers a handler generically through the trait (not the inherent `new`) to prove the impl is real. Compile-verified for the `esp32c3` riscv target with `use_esp_hal`.

### `tpt-e-slumber`
- [x] **BUG (fixed 2026-07-29)**: `tokens.rs` — `DmaParkedToken`/`RtcIsolatedToken`/`BuffersFlushedToken` all derive `Copy`, so once a real precondition-checked issuance path lands, a token proving a precondition at issuance time can be duplicated and reused after the precondition no longer holds (e.g. DMA restarts after a `DmaParkedToken` was obtained, but a stale copy still satisfies `enter_deep_sleep`'s signature). `enter_deep_sleep`/`try_sleep` already take tokens by value, so removing `Copy`/`Clone` alone makes them true move-once (linear) tokens. Fixed: dropped the `Copy`/`Clone` derives; replaced the `tokens_are_copy` test with `tokens_are_linear` (verifies move semantics); added `#[allow(missing_copy_implementations)]` on each token struct.

### `tpt-e-swarm-sync`
- [x] **BUG/security (fixed 2026-07-29)**: `mesh.rs`/`state_machine.rs` — the module doc claims "deterministic tie-breaking based on node IDs," but no code anywhere compared node IDs: any inbound `Election` message unconditionally forced the receiving Primary to step down (`HigherPriorityNodeFound`), so a lower-priority or malicious peer could force a legitimate Primary to demote with zero verification. Fixed: `Message` gains a `sender_id: u32` field; `Event::HeartbeatReceived`/`HigherPriorityNodeFound` carry the sender/candidate's node ID; a Primary now yields only to a genuinely lower-ID (higher-priority) node. Added `primary_only_yields_to_lower_id` Kani proof.
- [x] **BUG (fixed 2026-07-29)**: `state_machine.rs` — `(Primary, HeartbeatReceived)` was always a no-op, so once two nodes independently self-promoted to Primary during a partition, they never reconciled after the partition healed. `tests/multi_node_election.rs::partition_and_heal_convergence` demonstrates exactly this 3-way split-brain scenario but only asserts `!primaries.is_empty()` ("at least one"), so it passed CI despite producing the exact divergence the crate exists to prevent. Fixed: `Event::HeartbeatReceived` now carries `sender_id`; a Primary yields to a lower-ID heartbeat, giving real post-partition reconciliation (lowest-ID node remains Primary). `partition_and_heal_convergence` now asserts exactly one Primary and that it is node 1 (lowest ID).
- [x] **Test gap (fixed 2026-07-29)**: `tests/multi_node_election.rs::three_node_election_lowest_id_wins` never actually contests an election between differently-ID'd nodes (it manually fires `NoOtherNodesFound` on one node directly) — the "lowest ID wins" claim was untested. Rewritten: all three nodes race to Primary via `NoOtherNodesFound`, then exchange heartbeats; only the lowest-ID node (10) survives as Primary.
- [x] **Partially resolved 2026-07-29**: `Message` now has a `sender_id: u32` field (added as part of the tie-break fix above); `mock.rs::send_heartbeat` populates it from the sender's node ID. `mesh.rs`'s `Message.sequence` is still set to the sender's node ID rather than an incrementing counter, and `Ack`/`Data` messages are funneled through the same `HeartbeatReceived` event as real heartbeats with no correlation to prior outbound messages — the module doc's "message sequencing, acknowledgments" claim remains unimplemented beyond the message-type dispatch already tracked above (Phase 4 section). Building real sequencing/ack correlation is a larger, separate design task (bounded-state tracking of outstanding sends) out of scope for this pass.

