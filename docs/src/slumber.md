# tpt-e-slumber

Compile-time verified power management and sleep transitions.

## Overview

`tpt-e-slumber` uses a proof-token API to gate deep sleep transitions.
Subsystems issue tokens (e.g., `DmaParkedToken`, `RtcIsolatedToken`) only
when they can prove their precondition is satisfied. `SleepController::enter_deep_sleep`
requires all tokens as parameters — missing tokens produce a compile error.

## Key Types

- `SleepController` — Manages power transitions
- `DmaParkedToken` — Proves all DMA channels are parked
- `RtcIsolatedToken` — Proves RTC memory is isolated
- `BuffersFlushedToken` — Proves all TX buffers are flushed
- `MockSleepBackend` — Recordable sleep backend for testing (feature = "mock")

## Example

```rust
use tpt_e_slumber::sleep::SleepController;
use tpt_e_slumber::tokens::{DmaParkedToken, RtcIsolatedToken, BuffersFlushedToken};

let controller = SleepController::new();
let dma = DmaParkedToken::mock();
let rtc = RtcIsolatedToken::mock();
let buf = BuffersFlushedToken::mock();

// This would enter deep sleep (returns `!`):
// controller.enter_deep_sleep(dma, rtc, buf);
```

## Compile-time Safety

```rust,compile_fail
use tpt_e_slumber::sleep::SleepController;

let controller = SleepController::new();
controller.enter_deep_sleep(); // Error: missing all three tokens
```
