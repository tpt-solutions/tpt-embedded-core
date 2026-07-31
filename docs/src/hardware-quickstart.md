# Hardware Quickstart

[Getting Started](./getting-started.md) covers host-side (`mock`-feature)
testing, which needs no hardware at all. This page is the other half: the
fastest path from a fresh machine to real code running on a real ESP32
board, plus the toolchain gotchas that cost real debugging time to
discover — collected here once instead of re-derived per person.

The fastest-known-good path is **ESP32-C3 (RISC-V)**. It uses the default
`stable` Rust toolchain — no Xtensa toolchain, no version bisection. If
your board is a plain ESP32 or ESP32-S3 (Xtensa), see
[Xtensa boards](#xtensa-boards-esp32--esp32-s3) below; it's a longer path.

Not sure which family your board is? Espressif's own line splits like this:

| Chip | Architecture | Toolchain |
|---|---|---|
| ESP32-C3, ESP32-C6, ESP32-H2 | RISC-V | plain `rustup`, no extra install |
| ESP32 (original), ESP32-S2, ESP32-S3 | Xtensa | `espup`, a separate toolchain |

## ESP32-C3 in about 5 minutes

1. **Install the RISC-V target:**

   ```bash
   rustup target add riscv32imc-unknown-none-elf
   ```

2. **Install `espflash`, pinned to `3.x`.** This matters, not just a
   preference: as of this writing, `espflash 4.5.0`'s bundled bootloader
   has an `efuse_blk_rev` check that rejects real ESP32-C3 (rev v0.4)
   boards outright (`Image requires efuse blk rev >= v287.87, but chip is
   v1.3`, then a reboot loop) — even with `--ignore-app-descriptor`.
   `espflash 3.3.0` flashes and boots the same board correctly.

   ```bash
   cargo install espflash --version "^3"
   ```

3. **Flash the AES-DMA hardware smoke test** — this drives the chip's real
   AES peripheral through a real DMA channel and checks the result against
   a known-answer vector, so a pass means real hardware DMA + crypto
   actually worked, not just "the board turned on":

   ```bash
   cd firmware
   cargo run --release -p aes-dma-smoke
   ```

   Expected output over serial:

   ```
   AES-DMA PASS: hardware ciphertext matches FIPS-197 vector
   ```

If `cargo run` hangs waiting for a port or the monitor shows nothing,
double-check the board enumerated as a serial device (Windows Device
Manager / `ls /dev/tty*`) before assuming the firmware is at fault.

## Xtensa boards (ESP32 / ESP32-S3)

Xtensa chips need a *different* Rust toolchain entirely (`espup`, not
`rustup target add`), and that toolchain has version-compatibility traps
that cost real time to bisect the first time:

- **`espup`'s default (latest) Xtensa Rust toolchain doesn't compile
  `esp-hal = "0.22"`.** Its pinned `xtensa-lx-rt = "0.17.2"` uses
  pre-`naked_asm!` syntax that recent nightly Rust hard-errors on. Bisecting
  to an older Xtensa toolchain just trades this for a different failure
  (`indexmap`/`toml_edit`/`hashbrown` need `edition2024`, unstable before
  upstream Rust 1.85 — which is also where the naked-fn restriction starts,
  so no version satisfies both). **Fix: use `esp-hal = "0.23"`** (pulls in
  `xtensa-lx-rt 0.18.0`, already on `naked_asm!`) with espup's latest
  toolchain — no pinning needed. Note this means Xtensa firmware crates in
  this repo are on a different `esp-hal` version than the RISC-V ones
  (`esp-hal`'s `links = "esp-hal"` key forbids mixing versions in one
  workspace, which is why `firmware/hil-hello` is its own standalone
  workspace, not a member of `firmware/Cargo.toml`).
- **`esp-hal = "0.23"` moved `usb_serial_jtag`/`delay`/etc. behind an
  `unstable` feature** and dropped `prelude`/`#[entry]` in favor of
  `esp_hal::main`/`#[main]`. `default-features = false` (needed to select a
  chip feature) also disables `unstable`, so re-add it explicitly:
  `features = ["esp32s3", "unstable"]`.
- **The board may need a manual BOOT+RESET to even enumerate as USB.** One
  ESP32-S3 board here showed up as no USB device at all until: hold BOOT,
  tap RESET, release BOOT.
- **`--before usb-reset` is required for native USB-Serial-JTAG, not the
  default `default-reset`.** Without it, `espflash flash`/`monitor` both
  connect and report correct chip info, but the flashed app never actually
  starts (chip stays in the ROM bootloader) — easy to misdiagnose as a dead
  board, since the flash step reports success either way.
- Install with: `cargo install espup --locked && espup install --targets esp32s3` (swap the target for `esp32` as needed).

Full details, including a rustup-corruption recovery step encountered along
the way, are in
[`firmware/hil-hello/README.md`](https://github.com/tpt-solutions/tpt-embedded-core/blob/master/firmware/hil-hello/README.md).

## What's next

Both paths above prove the toolchain/build/flash/monitor pipeline and (for
`aes-dma-smoke`) one real hardware peripheral — they don't yet exercise the
five library crates' own code on real silicon (see the root `README.md`'s
[Status](../../README.md#status) section for exactly what is and isn't
hardware-validated today). For host-side development against the library
crates themselves, see [Getting Started](./getting-started.md) and
[Cross-Crate Wiring](./cross-crate-wiring.md).
