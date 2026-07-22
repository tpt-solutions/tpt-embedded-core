# `tpt-embedded-core`

[![Build](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/build.yml/badge.svg)](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/build.yml)
[![Tests](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/test.yml/badge.svg)](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/test.yml)
[![Proptest](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/proptest.yml/badge.svg)](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/proptest.yml)
[![Kani](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/kani.yml/badge.svg)](https://github.com/tpt-software/tpt-embedded-core/actions/workflows/kani.yml)

A proof-native, formally verified `no_std` foundation for ESP32 ecosystems.

## Philosophy

- **Typestate over Runtime Checks** — Invalid states are unrepresentable at compile time.
- **`#![deny(unsafe_code)]`** — Unsafe code is isolated, documented, and formally verified.
- **Property-Based Testing** — Core invariants proven by fuzzing and model checking.
- **WCET-Bounded** — Every public function has a deterministic Worst-Case Execution Time.

## Architecture

```text
tpt-embedded-core/
├── tpt-e-typestate-hal/     # [Foundation] Compile-time safe DMA & ISR abstractions
├── tpt-e-chronos/           # [Data] WCET-bounded telemetry ring buffer
├── tpt-e-cipher/            # [Security] Formally verified HW crypto wrappers
├── tpt-e-slumber/           # [Power] Compile-time verified sleep transitions
└── tpt-e-swarm-sync/        # [Network] Verified mesh coordination state machine
```

### Crate Dependencies

| Crate | Depends On | Status |
|---|---|---|
| `tpt-e-typestate-hal` | — (foundation) | Phase 1 |
| `tpt-e-chronos` | `tpt-e-typestate-hal` (DMA handles) | Phase 1 |
| `tpt-e-slumber` | `tpt-e-typestate-hal` (state tokens) | Phase 2 |
| `tpt-e-cipher` | `tpt-e-typestate-hal` (DMA handles) | Phase 3 |
| `tpt-e-swarm-sync` | `tpt-e-chronos` (message queuing) | Phase 4 |

## Getting Started

Run the full test suite (all crates, host-side, via the `mock` feature —
there is no hardware backend to test against yet, see [Status](#status)
below):

```bash
cargo test --features mock --workspace
```

Each crate has a small, runnable example under its `examples/` directory.
They all require `--features mock` since they use host-side mock backends
rather than real hardware peripherals:

```bash
cargo run -p tpt-e-typestate-hal --example dma_transfer  --features mock
cargo run -p tpt-e-chronos       --example ring_buffer_basics --features mock
cargo run -p tpt-e-slumber       --example sleep_cycle    --features mock
cargo run -p tpt-e-cipher        --example hash_a_buffer  --features mock
cargo run -p tpt-e-swarm-sync    --example mesh_election  --features mock
```

### Walkthrough: wiring the crates together

This sketches how the five crates are meant to compose in a real firmware
image. It's mock-backed end to end (nothing here talks to real hardware
yet — see [Status](#status)), so treat it as a reference for the shape of
the integration, not a deployable example:

```rust
use tpt_e_chronos::ring_buf::RingBuf;
use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_slumber::sleep::SleepController;
use tpt_e_slumber::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

// 1. tpt-e-chronos: buffer telemetry samples ISR-safely.
let mut telemetry: RingBuf<u32, 32> = RingBuf::new(0);
telemetry.push(read_sensor()).ok();

// 2. tpt-e-typestate-hal: hand the buffer to DMA through the typestate
//    chain — invalid transitions (e.g. starting a transfer before
//    configuring) are compile errors, not runtime checks.
let channel = DmaChannel::mock(0)
    .configure(dma_target_buffer(), 64)
    .start()
    .wait();

// 3. tpt-e-cipher: hash the batch before it goes over the wire.
// (see `cargo run -p tpt-e-cipher --example hash_a_buffer --features mock`)

// 4. tpt-e-swarm-sync: coordinate with other mesh nodes.
// (see `cargo run -p tpt-e-swarm-sync --example mesh_election --features mock`)

// 5. tpt-e-slumber: once DMA is parked, RTC is isolated, and buffers are
//    flushed, enter deep sleep — missing a token is a compile error.
let controller = SleepController::new();
// let dma_token = /* minted by tpt-e-typestate-hal once real precondition
//                    tracking lands */;
// controller.enter_deep_sleep(dma_token, rtc_token, buffers_token);
# fn read_sensor() -> u32 { 0 }
# fn dma_target_buffer() -> &'static mut [u8] { Box::leak(vec![0u8; 64].into_boxed_slice()) }
```

### Status

This is early-stage software. In particular:

- No crate has been validated against real ESP32 hardware yet — every test
  and example above runs against `mock` (host-side, `std`-backed) backends.
  See `todo.md` for the current per-crate gap list.
- `tpt-e-cipher`'s SHA-256 is a real, tested implementation; its AES and
  ECC engines are still placeholders (real constant-time AES needs a
  bitsliced or hardware-backed implementation, not naive table lookups —
  see `todo.md`).
- `tpt-e-slumber`'s proof tokens are not yet wired to real precondition
  checks in `tpt-e-typestate-hal` — today they only prevent forging tokens
  from outside the crate.

## Versioning & Publishing

All crates in this workspace share a single workspace version (`0.1.0`). Each
crate is independently publishable to crates.io but follows a synchronized
versioning strategy during early development (Phase 0–4). Post-stabilization,
breaking changes to `tpt-e-typestate-hal` will trigger a minor version bump
for all dependent crates.

## License

Dual-licensed under MIT OR Apache-2.0.
