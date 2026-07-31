# `tpt-e-slumber`

Compile-time verified power management and sleep transitions for ESP32.

Part of [`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core),
a proof-native `no_std` foundation for ESP32 ecosystems.

## What it does

Gates deep-sleep entry behind a proof-token API: subsystems issue tokens
(`DmaParkedToken`, `RtcIsolatedToken`, `BuffersFlushedToken`) only when they
can prove their precondition holds, and `SleepController::enter_deep_sleep`
requires all of them as parameters — a missing token is a compile error,
not a runtime check. Tokens are linear (not `Copy`/`Clone`), so a token
can't be reused after the precondition it proved no longer holds.

With the `use_esp_hal` feature (plus a chip feature: `esp32`/`esp32s3`/
`esp32c3`/`esp32c6`), `enter_deep_sleep` wires to the real `esp-hal` RTC
deep-sleep instruction — flashed and confirmed on a real ESP32-C3 (see the
repo's `firmware/slumber-smoke`).

## Example

```rust
use tpt_e_slumber::sleep::SleepController;
use tpt_e_slumber::tokens::{DmaParkedToken, RtcIsolatedToken, BuffersFlushedToken};

let controller = SleepController::new();
let dma = DmaParkedToken::mock();
let rtc = RtcIsolatedToken::mock();
let buf = BuffersFlushedToken::mock();
// controller.enter_deep_sleep(dma, rtc, buf); // returns `!`
```

Run it: `cargo run -p tpt-e-slumber --example sleep_cycle --features mock`

## License

Dual-licensed under MIT OR Apache-2.0. See the
[repository root](https://github.com/tpt-solutions/tpt-embedded-core) for
full docs, architecture, and the other four crates.
