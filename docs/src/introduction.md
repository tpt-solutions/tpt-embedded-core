# TPT Embedded Core

Compile-time verified, no-std embedded Rust abstractions for the ESP32 family.

`tpt-embedded-core` is a workspace of five crates that together provide a
safety-critical foundation for ESP32 embedded development. Every crate
enforces its core invariants at **compile time** using Rust's type system —
runtime checks are replaced by typestate patterns, proof tokens, and
zero-cost abstractions.

## Crates

| Crate | Purpose |
|-------|---------|
| [`tpt-e-typestate-hal`](./typestate-hal.md) | Typestate-enforced DMA & ISR abstractions |
| [`tpt-e-chronos`](./chronos.md) | WCET-bounded ring buffer for time-series data |
| [`tpt-e-cipher`](./cipher.md) | Constant-time crypto wrappers (AES, SHA-256, ECC) |
| [`tpt-e-slumber`](./slumber.md) | Proof-token-gated sleep transitions |
| [`tpt-e-swarm-sync`](./swarm-sync.md) | Deterministic mesh coordination state machine |

## Design Principles

1. **Typestate over runtime checks** — invalid state transitions are compile errors
2. **`deny(unsafe_code)`** — unsafe is isolated to documented, minimal boundary modules
3. **Property-based testing** — invariants verified via `proptest` with randomized inputs
4. **Formal verification** — critical paths proven with `cargo kani`
5. **WCET-bounded** — all hot-path operations execute in O(1) time

## Getting Started

```toml
[dependencies]
tpt-e-chronos = { version = "0.1", features = ["mock"] }
```

```rust
use tpt_e_chronos::ring_buf::RingBuf;

let buf = RingBuf::<u32, 8>::new(0);
assert!(buf.push(42).is_ok());
assert_eq!(buf.pop(), Some(42));
```

See the [Getting Started](./getting-started.md) guide for a full walkthrough.
