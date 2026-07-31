# `{{project-name}}`

Battery-powered ESP32 mesh sensor node scaffold built on
[`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core).

Unlike the generic [`templates/esp32-app`](../esp32-app) starter, this
template scaffolds one specific, realistic shape: a node that wakes,
samples a sensor, coordinates its role with the rest of the mesh, and goes
back to sleep — the pattern `tpt-e-swarm-sync` exists to support.

- **`tpt-e-swarm-sync`** — join/maintain the mesh's Primary/Secondary
  election each wake cycle (`MeshStateMachine`)
- **`tpt-e-chronos`** — ISR-safe telemetry buffering between wake cycles
  (`RingBuf<u32, N>`)
- **`tpt-e-slumber`** — proof-token gated deep sleep between cycles
  (placeholder tokens in this template — see "Customising" below)

## Prerequisites

- Rust stable (see `rust-toolchain.toml`)
- `espflash` (`cargo install espflash --version "^3"` for RISC-V chips)
- RISC-V target: `rustup target add riscv32imc-unknown-none-elf`

## Usage

```bash
cargo generate \
  --git https://github.com/tpt-solutions/tpt-embedded-core \
  --branch master \
  --init \
  --name my-swarm-node \
  templates/swarm-node-app
```

Or copy this template directory into a new crate directly — it depends on
`tpt-embedded-core` via a `git` dependency pinned to a specific commit (not
`path`, so it builds standalone outside a clone of the monorepo). Update the
`rev` in `Cargo.toml` to pick up newer fixes, or switch to published
crates.io versions once they exist.

## Running on host

```bash
cargo test --features mock
```

## Flashing to ESP32-C3

```bash
cargo run --release
```

## Customising

1. Give each physical node a distinct `NODE_ID` in `src/main.rs` — election
   tie-breaking is by lowest ID, so duplicate IDs break the mesh's
   single-Primary guarantee.
2. Replace `read_sensor()` with a real ADC/I2C/SPI driver.
3. Wire a real ESP-NOW (or 802.15.4) driver's receive handler to call
   `mesh.process_event(Event::HeartbeatReceived { sender_id })` instead of
   this scaffold's always-`NoOtherNodesFound` path, and use
   `tpt_e_swarm_sync::mesh::MeshNode` (which wraps its own outbound/inbound
   `tpt-e-chronos` ring buffers) to actually send/receive `Message`s rather
   than just draining the telemetry buffer locally.
4. Enable `tpt-e-slumber`'s `use_esp_hal` + chip feature and replace the
   mock tokens with real precondition-checked ones once that wiring exists
   upstream (see `tpt-e-slumber`'s "Known limitations"), then uncomment the
   `enter_deep_sleep` call.
5. Change the `esp-hal` chip feature in `Cargo.toml` / `.cargo/config.toml`
   to match your board, same as `templates/esp32-app`.
