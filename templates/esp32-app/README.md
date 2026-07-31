# `{{project-name}}`

Starter application scaffold built on [`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core).

This template demonstrates the core patterns of the five library crates:

- **`tpt-e-typestate-hal`** — typestate-gated DMA channel (`Idle → Configured → Transferring → Complete`)
- **`tpt-e-chronos`** — ISR-safe ring buffer (`RingBuf<u32, N>`)
- **`tpt-e-cipher`** — SHA-256 hashing (`MockSha256Engine`) over ring-buffer data
- **`tpt-e-slumber`** — proof-token gated deep sleep (placeholder in template)

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
  --name my-esp32-app \
  templates/esp32-app
```

Or copy this template directory into a new crate directly — it depends on
the five `tpt-embedded-core` crates via a `git` dependency pinned to a
specific commit (not `path`, so it builds standalone outside a clone of the
monorepo). Update the `rev` in `Cargo.toml` to pick up newer fixes, or
switch to published crates.io versions once they exist.

## Running on host

```bash
cargo test --features mock
```

## Flashing to ESP32-C3

```bash
cargo run --release
```

## Customising

1. Change the `esp-hal` chip feature in `Cargo.toml` to match your board
   (`esp32`, `esp32s3`, `esp32c6`, `esp32c3`).
2. Update `.cargo/config.toml`'s `target` and `runner` if you're targeting
   a different chip family.
3. Replace the `loop {}` at the end of `main()` with your application logic.
4. For real hardware crypto, enable `tpt-e-typestate-hal`'s `use_esp_hal`
   and chip features and construct a real `AesDmaChannel`.
