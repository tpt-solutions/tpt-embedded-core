# tpt-e-typestate-hal

Compile-time safe DMA & ISR abstractions for the ESP32 family.

## Overview

`tpt-e-typestate-hal` is the foundational HAL abstraction crate. It defines
the typestate chain `Idle → Configured → Transferring → Complete` as
distinct marker types, ensuring invalid DMA operations are caught at compile
time.

## Key Types

- `DmaChannel<State, B>` — A DMA channel parameterized by its current state
- `IsrGuard<F, B>` — An ISR registration guard (RAII: drop unregisters)
- `MockDmaChannel` — Host-side mock for testing (feature = "mock")
- `EspHalBackend` — Real hardware backend (feature = "use_esp_hal")

## Features

| Feature | Description |
|---------|-------------|
| `mock` | Enables `MockDmaChannel` and `MockIsrGuard` for host-side testing |
| `use_esp_hal` | Enables `EspHalBackend` for real hardware |
| `esp32` | Forwards to `esp-hal/esp32` |
| `esp32s3` | Forwards to `esp-hal/esp32s3` |
| `esp32c3` | Forwards to `esp-hal/esp32c3` |
| `esp32c6` | Forwards to `esp-hal/esp32c6` |

## Example

```rust
use tpt_e_typestate_hal::dma::DmaChannel;

let channel = DmaChannel::<_, tpt_e_typestate_hal::mock::MockDmaChannel>::mock(0);
let configured = channel.configure(buf, 64);
let transferring = configured.start();
let complete = transferring.wait();
```

## Safety

This crate contains unavoidable `unsafe` blocks at the hardware boundary.
These are isolated in `backend.rs` with documented safety invariants.
Exception granted per workspace policy: this is the foundational HAL
boundary crate.
