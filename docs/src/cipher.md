# tpt-e-cipher

Constant-time crypto wrappers for ESP32 hardware accelerators.

## Overview

`tpt-e-cipher` provides trait-based abstractions over ESP32 crypto
peripherals (AES, SHA-256, ECC). The API guarantees constant-time
execution — no data-dependent branching or timing variations.

## Key Types

- `Sha256Engine` — SHA-256 hasher (software fallback pending hardware wiring)
- `MockAesEngine` — XOR-based mock AES for logic testing (NOT constant-time)
- `MockSha256Engine` — Software SHA-256 for host-side testing

## Traits

- `Aes` — Constant-time AES block encryption
- `Sha256` — Incremental SHA-256 hashing

## Status

| Component | Status |
|-----------|--------|
| SHA-256 | Software implementation, verified against NIST KATs |
| AES | Mock (XOR-based, NOT constant-time) — real implementation pending |
| ECC | Placeholder only — needs hardware-backed implementation |

## Example

```rust
use tpt_e_cipher::sha::Sha256Engine;
use tpt_e_cipher::traits::Sha256;

let mut hasher = Sha256Engine::new();
hasher.update(b"hello");
let digest = hasher.finalize();
assert_eq!(digest.len(), 32);
```

## Safety

The mock implementations are **not** constant-time and must not be used
for processing secret data in production. Timing analysis must be performed
against the hardware implementation.
