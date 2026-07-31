# Philosophy

The TPT (Typestate-Per-Time) standard governs how every crate in this
workspace is designed, implemented, tested, and reviewed.

## Core Tenets

### Typestate Pattern

State transitions are represented as distinct zero-sized types. The Rust
compiler enforces that you cannot, for example, start a DMA transfer
without first configuring the channel — because the types don't allow it.

```rust,compile_fail
use tpt_e_typestate_hal::dma::DmaChannel;

// This fails to compile: Idle → Transferring is not a valid transition.
let channel = DmaChannel::<_, MockDmaChannel>::mock(0);
let _transferring = channel.start(); // ERROR: no method `start` on `DmaChannel<Idle>`
```

### Minimal Unsafe

Each crate documents exactly why `unsafe` is needed and isolates it in a
dedicated module. The workspace policy is `#![deny(unsafe_code)]` at the
crate root, with a documented exception process for unavoidable hardware
boundary code.

### Property-Based Testing

Every public API is tested not just with examples, but with `proptest` —
randomized inputs that exercise edge cases the author didn't think of.

### Formal Verification

Critical invariants (ring buffer bounds, typestate transitions, state
machine divergence) are proven with `cargo kani` — a model checker that
explores all possible inputs.

### WCET Bounding

All hot-path operations (`push`, `pop`, state transitions) execute in O(1)
time with no data-dependent branching, no allocation, and no locks beyond
minimum-length critical sections.

## See Also

- [`CONTRIBUTING.md`](https://github.com/tpt-solutions/tpt-embedded-core/blob/master/CONTRIBUTING.md) — full TPT Standard as a review checklist
- [`spec.txt`](https://github.com/tpt-solutions/tpt-embedded-core/blob/master/spec.txt) — formal specification
