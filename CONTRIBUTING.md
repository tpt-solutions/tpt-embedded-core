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
