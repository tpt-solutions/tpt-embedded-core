# `tpt-e-typestate-hal`

Compile-time safe DMA & ISR abstractions for the ESP32 family.

Part of [`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core),
a proof-native `no_std` foundation for ESP32 ecosystems. This is the
foundational crate the other four (`tpt-e-chronos`, `tpt-e-cipher`,
`tpt-e-slumber`, `tpt-e-swarm-sync`) build on.

## What it does

Defines the typestate chain `Idle → Configured → Transferring → Complete`
as distinct marker types, so an invalid DMA operation (e.g. starting a
transfer before configuring the channel) is a compile error, not a runtime
check.

- `DmaChannel<State, B>` — a DMA channel parameterized by its current state
- `IsrGuard<F, B>` — an ISR registration guard (RAII: drop unregisters)
- `MockDmaChannel` / `MockIsrGuard` — host-side (`std`-backed) fakes for
  testing without hardware, behind the `mock` feature
- `AesDmaChannel` — a peripheral-specific typestate wrapper around
  `esp-hal`'s real hardware AES-DMA, validated against the FIPS-197
  known-answer vector on a real ESP32-C3 (see the repo's `firmware/`
  directory for the smoke test)

## Status

Real hardware validation exists for the AES-DMA path specifically (above).
The generic `EspHalBackend`/`DmaChannel` hardware path is still a stub —
see the workspace root's `todo.md` for the current per-crate gap list and
rationale.

## Example

```rust
use tpt_e_typestate_hal::dma::DmaChannel;

let channel = DmaChannel::<_, tpt_e_typestate_hal::mock::MockDmaChannel>::mock(0);
let configured = channel.configure(buf, 64);
let transferring = configured.start();
let complete = transferring.wait();
```

Run it: `cargo run -p tpt-e-typestate-hal --example dma_transfer --features mock`

## License

Dual-licensed under MIT OR Apache-2.0. See the
[repository root](https://github.com/tpt-solutions/tpt-embedded-core) for
full docs, architecture, and the other four crates.
