# `esp32-smoke`

Minimal flash-and-run smoke test for the original ESP32 (Xtensa, not S3/S2/C3/C6).

Like `../hil-hello`, this exists solely to prove the
toolchain/build/flash/monitor pipeline works end to end against real
ESP32 silicon. Unlike the RISC-V firmware crates in this workspace
(`aes-dma-smoke`, `slumber-smoke`, `typestate-aes-dma-smoke`), which
exercise actual peripheral DMA and library-crate APIs, this is a
minimal "alive" counter — the original ESP32 has no built-in USB
Serial/JTAG, so it prints over UART0 (GPIO1 TX / GPIO3 RX) instead.

It does not exercise any of the five library crates yet — it just prints
an incrementing counter, proving real code built from this toolchain
boots and runs.

## Why this crate exists

The unchecked "Validate against target chips" item in the root `todo.md`
documents that ESP32-C3 and ESP32-S3 milestones were both reached on
2026-07-29, but plain ESP32 (original Xtensa) has not yet been flashed.
This crate is the software half of that gap: it compiles for
`xtensa-esp32-none-elf` and is ready to flash as soon as a board and
USB-UART bridge are attached.

## Flashing

Requires the Xtensa Rust toolchain (see `rust-toolchain.toml`) and
`espflash` on `PATH`.

```bash
cd firmware/esp32-smoke
cargo run --release
```

If the board's UART0 is wired to a USB-UART bridge (the standard
ESP32-DevKitC layout: GPIO1 = TX, GPIO3 = RX, EN = reset), `espflash`
will flash and monitor automatically.

For non-default ports or reset strategies:

```bash
espflash flash --port COM6 --before default-reset --after hard-reset \
  target/xtensa-esp32-none-elf/release/esp32-smoke
espflash monitor --port COM6
```

## Relationship to `hil-hello`

`hil-hello` (sibling in `../hil-hello`) already contains an `esp32`
feature that switches its output from USB Serial/JTAG to UART0. That
crate uses `esp-hal = "0.23"` because `0.22`'s pinned `xtensa-lx-rt`
does not compile under the Xtensa toolchain versions that also support
`edition2024`. This crate follows the same pattern.
