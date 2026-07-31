# `tpt-e-chronos`

WCET-bounded, zero-allocation telemetry ring buffer for ESP32.

Part of [`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core),
a proof-native `no_std` foundation for ESP32 ecosystems.

## What it does

A heapless, const-generic ring buffer for time-series/telemetry data in
ISR/main-loop architectures, with a critical-section-guarded push/pop path
(via the `critical-section` crate) and zero-copy DMA handoff into
`tpt-e-typestate-hal`.

- `RingBuf<T, CAP>` — ISR-safe ring buffer, `CAP` enforced power-of-two at
  compile time
- `DmaLoan<'a, T, CAP>` — exclusive-borrow loan handle for zero-copy DMA
  transfer (the borrow checker rejects `push`/`pop` while a loan is live)
- Proven by proptest (push/pop interleavings never lose or corrupt data)
  and Kani (panic-freedom under any interleaving)

## Example

```rust
use tpt_e_chronos::ring_buf::RingBuf;

let mut buf = RingBuf::<u32, 8>::new(0);
assert!(buf.push(42).is_ok());
assert_eq!(buf.pop(), Some(42));
```

Run it: `cargo run -p tpt-e-chronos --example ring_buffer_basics --features mock`

## License

Dual-licensed under MIT OR Apache-2.0. See the
[repository root](https://github.com/tpt-solutions/tpt-embedded-core) for
full docs, architecture, and the other four crates.
