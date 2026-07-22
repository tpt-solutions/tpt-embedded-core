# tpt-e-chronos

WCET-bounded, zero-allocation ring buffer for ESP32 telemetry data.

## Overview

`tpt-e-chronos` provides a heapless, const-generic ring buffer designed for
time-series data in ISR/main-loop architectures. It supports zero-copy DMA
handoff via `tpt-e-typestate-hal`.

## Key Types

- `RingBuf<T, CAP>` — Lock-free ring buffer with power-of-two capacity
- `DmaLoan<'a, T, CAP>` — Loan handle for zero-copy DMA transfer
- `MockClock` — Manually-advancing clock for deterministic testing (feature = "mock")

## Guarantees

- **Zero-allocation**: Fixed-size array, no heap
- **WCET-bounded**: O(1) push/pop, no data-dependent branching
- **ISR-safe**: Atomic head/tail with acquire/release ordering
- **FIFO-ordered**: Elements dequeued in enqueue order

## Example

```rust
use tpt_e_chronos::ring_buf::RingBuf;

let buf = RingBuf::<u32, 8>::new(0);
assert!(buf.push(42).is_ok());
assert_eq!(buf.pop(), Some(42));
```

## DMA Integration

With `tpt-e-typestate-hal` enabled:

```rust,ignore
use tpt_e_chronos::dma_handoff::transfer_with_dma;

let mut ring = RingBuf::<u32, 4>::new(0);
let _ = ring.push(42);
let channel = DmaChannel::<_, MockDmaChannel>::mock(0);
// Lend → DMA transfer → reclaim in one call
let mut ring = unsafe { transfer_with_dma(&mut ring, channel) };
assert_eq!(ring.pop(), Some(42));
```
