# Devcontainer for tpt-embedded-core.
#
# Targets the RISC-V (ESP32-C3/C6) development path — `rustup target add`
# only, no `espup`/Xtensa toolchain — because that path is the fast,
# low-friction one (see docs/src/hardware-quickstart.md) and Xtensa's own
# toolchain is large, fragile, and version-sensitive enough (see that same
# doc's "Xtensa boards" section and firmware/hil-hello/README.md) that
# baking it into every contributor's container image is the wrong default.
# Add it yourself with `espup install --targets esp32,esp32s3` inside the
# container if you're working on an Xtensa board.
#
# This also gives Kani formal verification a working Linux host: Kani does
# not build on native Windows (see the root todo.md's swarm-sync section,
# "Kani doesn't build on native Windows... needs CI to confirm"), so this
# container is the first non-CI place those proofs can actually run.
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Pinned per docs/src/hardware-quickstart.md: espflash 4.5.0's bundled
# bootloader rejects real ESP32-C3 (rev v0.4) boards outright
# (efuse_blk_rev mismatch) even with --ignore-app-descriptor; 3.x flashes
# and boots correctly.
ARG ESPFLASH_VERSION="^3"

RUN rustup target add riscv32imc-unknown-none-elf riscv32imac-unknown-none-elf \
    && cargo install espflash --version "${ESPFLASH_VERSION}" --locked \
    && cargo install cargo-generate --locked \
    && cargo install cargo-deny --locked \
    && cargo install --locked kani-verifier \
    && cargo kani setup
