# CI Pipeline

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `build.yml` | Push/PR to main | Build matrix (4 chips + default features + cargo-deny) |
| `test.yml` | Push/PR to main | `cargo test --workspace --features mock` |
| `proptest.yml` | Push/PR to main | Property-based tests (10,000 cases) |
| `kani.yml` | Push/PR to main | Formal verification (workspace-wide + crypto-specific) |
| `hil.yml` | Nightly (disabled) | Hardware-in-loop on real ESP32-S3 |

## Build Matrix

The `build.yml` workflow tests against 4 target chips:

- `esp32` → `xtensa-esp32-none-elf`
- `esp32s3` → `xtensa-esp32s3-none-elf`
- `esp32c3` → `riscv32imc-unknown-none-elf`
- `esp32c6` → `riscv32imac-unknown-none-elf`

Each chip uses its real `esp-hal` feature flag, not a cosmetic matrix.

## Running Locally

```bash
# Full test suite
cargo test --workspace --features mock

# Property-based tests
cargo test --features mock --release

# Formal verification (requires kani)
cargo kani --workspace

# Crypto-specific Kani
cargo kani --features mock -p tpt-e-cipher
```
