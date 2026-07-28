# Contributing to `tpt-embedded-core`

All contributions must adhere to the **TPT Standard** outlined in the project spec (§4). This checklist is applied during code review.

## Review Checklist

### Typestate over Runtime Checks
- [ ] State transitions are encoded in the type system, not as runtime enums with match arms
- [ ] Invalid operations produce compile-time errors, not runtime panics or `Result::Err`
- [ ] Zero-sized marker types are preferred over boolean flags or runtime state tracking

### `#![deny(unsafe_code)]`
- [ ] All crates in the workspace have `#![deny(unsafe_code)]` at the crate root
- [ ] Any unavoidable `unsafe` block is isolated in a minimal, documented boundary module
- [ ] Each `unsafe` block includes a `// SAFETY:` comment justifying why the preconditions hold
- [ ] Exceptions to `deny(unsafe_code)` are documented per-crate with explicit spec-section references

### Property-Based Testing
- [ ] Core invariants are tested using `proptest` or equivalent framework
- [ ] At minimum: state-transition invariants, buffer bounds, and panic-freedom under interleaving
- [ ] Mock feature allows 100% of logic tests on host without hardware

### WCET-Bounded Design
- [ ] All public functions have deterministic, bounded execution time
- [ ] No unbounded loops, dynamic dispatch, or recursion in public API paths
- [ ] Loop bounds are either const-generic parameters or proven via Kani

### Verification
- [ ] Kani proofs accompany any module with safety-critical invariants (buffer bounds, typestate transitions, constant-time execution)
- [ ] Kani harnesses cover all reachable states and edge cases
- [ ] CI includes `cargo kani` runs for verified modules

### General
- [ ] `cargo test --features mock` passes
- [ ] Code compiles with `deny(unsafe_code)` and no warnings
- [ ] Public API is documented with rustdoc, including safety invariants where applicable
- [ ] `#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]` is enabled

## Crate Dependency Graph & Versioning Policy

`tpt-e-typestate-hal` is foundational; breaking changes to it can ripple
through the rest of the workspace. The current internal dependency graph:

```
tpt-e-typestate-hal   (no internal deps — foundational)
        ^  ^
        |  |
        |  +---- tpt-e-cipher      (optional dev-dep, for the DMA+crypto
        |                           integration test only — not a
        |                           production dependency)
        +------- tpt-e-chronos     (optional prod dep, for DMA handoff)
                        ^
                        |
                 tpt-e-swarm-sync  (required prod dep, for ring-buffer
                                    message queuing)

tpt-e-slumber   (no internal deps — standalone)
```

All internal deps are currently `path`-only (no `version` key), which only
resolves within this workspace. This is fine pre-publish, but is itself a
concrete blocker for the still-open "public release checklist" item: cargo
requires a `version` alongside `path` for a dependency to resolve once the
depending crate is published to crates.io, so path-only internal deps must
gain version constraints as part of that checklist, not before.

Versioning policy while all crates share the workspace-synced `0.1.0`
(see README's "Versioning & Publishing" section for the synchronized vs.
independent-post-stabilization split):

- A change to `tpt-e-typestate-hal`'s public API (traits in `backend.rs`,
  the `DmaChannel` typestate chain, `IsrGuard`) that breaks any downstream
  crate is caught immediately by `build-default-features` and the
  `--features mock` build jobs in CI (`build.yml`/`test.yml`) — both build
  every crate in the workspace, so a breaking change anywhere fails CI at
  the same PR, not silently downstream later.
- Because of that CI coverage, there is currently no separate manual
  "bump the dependents" step to remember: a PR that breaks a downstream
  crate simply won't pass CI until the downstream usage is updated in the
  same PR. This section exists to make that dependency shape (and the
  path-only-deps caveat above) explicit, not to add new process.
