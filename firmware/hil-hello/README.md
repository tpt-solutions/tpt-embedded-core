# `hil-hello`

First-ever flashable smoke test for `tpt-embedded-core`, confirming the
toolchain/build/flash/monitor pipeline works against real ESP32 silicon.
This is its own standalone one-package Cargo workspace (own `[workspace]`
table in `Cargo.toml`) — deliberately *not* a member of `../Cargo.toml`
(the shared `firmware/` workspace), and deliberately separate from the main
workspace at the repo root, so it can never affect `cargo build --workspace`
for the library crates. See also `../aes-dma-smoke`, a sibling crate (a
member of `../Cargo.toml`) that drives real hardware DMA on ESP32-C3.

It does not exercise any of the five library crates yet — it just prints an
incrementing counter over the chip's built-in USB Serial/JTAG controller,
proving real code built from this toolchain boots and runs.

Currently targets `esp32s3` (Xtensa). Previously validated against
`esp32c3` (RISC-V) — retargeting means changing `Cargo.toml`'s `esp-hal`
feature/version, `.cargo/config.toml`'s target triple, and (per the notes
below) the toolchain itself, since Xtensa and RISC-V chips need genuinely
different Rust toolchains. `esp32c3` used `esp-hal = "0.22"` with the
default `stable` Rust toolchain and target `riscv32imc-unknown-none-elf`.

## Why this crate isn't in `../Cargo.toml`'s workspace

Cargo forbids two different versions of any crate that declares
`links = "..."` from ever coexisting in one workspace's dependency graph —
even across binaries that would never actually be linked together. `esp-hal`
declares `links = "esp-hal"`, and the RISC-V (`aes-dma-smoke`, esp-hal 0.22)
and Xtensa (this crate, esp-hal 0.23.1) paths need different `esp-hal`
versions, so they cannot share a workspace.

## Flashing

Requires the Xtensa Rust toolchain (see "Known-good toolchain" below) and
its GCC/Clang bin directories on `PATH` (not just the default shell PATH —
see below):

```bash
cd firmware/hil-hello
export PATH="$HOME/.rustup/toolchains/esp/xtensa-esp-elf/bin:$HOME/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-clang/bin:$PATH"
cargo run --release
```

`cargo run` invokes `espflash flash --monitor`, which needs a real TTY for
its interactive prompts — if running from a non-interactive shell/CI/agent,
flash and monitor separately with explicit port and reset flags instead:

```bash
espflash flash --port COM6 --before usb-reset --after hard-reset \
  target/xtensa-esp32s3-none-elf/release/hil-hello
espflash monitor --port COM6 --before usb-reset
```

## Known-good toolchain (ESP32-S3 / Xtensa)

- **Board needs a manual BOOT+RESET to even enumerate as USB.** On the
  board used here, Windows shows *no* USB device at all until you: hold
  BOOT, tap RESET, release BOOT. Only then does it enumerate as
  `USB JTAG/serial debug unit` / `USB Serial Device (COMn)`. (This did not
  happen on the ESP32-C3 board — same cable/setup, no button-press needed
  there.)
- **`--before usb-reset` is required, not the default `default-reset`.**
  `espflash`'s default before/after-reset strategy toggles DTR/RTS as if
  talking to a classic USB-UART bridge chip. The native USB-Serial-JTAG
  peripheral needs its own reset sequence
  (`--before usb-reset --after hard-reset`); without it, `espflash flash`
  and `espflash monitor` both connect fine and report correct chip info,
  but the flashed app never actually starts producing UART output (chip
  stays in the ROM bootloader). This is easy to misdiagnose as a dead
  board or bad firmware — the flash step reports success either way.
- **`espup`'s default (latest) Xtensa Rust toolchain doesn't compile
  `esp-hal 0.22`.** `esp-hal = "0.22"`'s pinned `xtensa-lx-rt = "0.17.2"`
  uses the pre-`naked_asm!` naked-function syntax
  (`#[naked]` + `asm!` inside the function body), which recent nightly
  Rust hard-errors on (`asm! macro is not allowed in naked functions`).
  Bisecting to an older Xtensa Rust release just trades this error for a
  different one (`indexmap`/`toml_edit`/`hashbrown` in the dependency
  graph require `edition2024`, unstable before upstream Rust 1.85 — and
  1.85 already has the naked-fn restriction, so no `espup`-installable
  version satisfies both). **Fix: use `esp-hal = "0.23"` instead**
  (pulls in `xtensa-lx-rt 0.18.0`, which already uses `naked_asm!`) with
  espup's default/latest Xtensa Rust toolchain — no version pinning
  needed.
- **`esp-hal 0.23` moved `usb_serial_jtag`/`delay`/etc. behind an
  `unstable` cargo feature**, and dropped the `prelude` module + `#[entry]`
  macro in favor of `esp_hal::main` + `#[main]`. `default-features = false`
  (needed to pick the chip feature) also disables `unstable` (it's a
  *default* feature), so it must be re-added explicitly:
  `features = ["esp32s3", "unstable"]`.
- **`espup install` can hit `rust-std` component "detected conflict"
  errors** if a previous `rustup target add` for a RISC-V target
  (`riscv32imac-unknown-none-elf` etc.) was interrupted, leaving a
  manifest file / `components`-file entry with no matching tracked
  install. Symptom: `rustup component list --toolchain stable` doesn't
  show the target as `(installed)` even though its manifest and lib files
  exist on disk. Fix: manually delete the stale
  `lib/rustlib/manifest-rust-std-<target>` file and
  `lib/rustlib/<target>/` directory under the affected toolchain, and
  strip the target's line from `lib/rustlib/components`, then retry
  `rustup target add`.
- Install with: `cargo install espup --locked && espup install --targets esp32s3`.

## Known-good toolchain (ESP32-C3 / RISC-V)

- **`espflash` must be `3.x`, not the latest `4.5.0`.** As of this writing,
  `espflash 4.5.0`'s bundled bootloader has an `efuse_blk_rev` requirement
  that doesn't match a real ESP32-C3 (rev v0.4)'s actual efuse block
  revision, and the board reboot-loops with `Image requires efuse blk rev
  >= v287.87, but chip is v1.3` / `Factory app partition is not bootable` —
  even with `--ignore-app-descriptor`. `espflash 3.3.0` (contemporaneous
  with `esp-hal 0.22`, which this repo is pinned to) flashes and boots
  correctly. Install with `cargo install espflash --version "^3"`.
