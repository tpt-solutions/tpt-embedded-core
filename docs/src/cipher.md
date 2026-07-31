# tpt-e-cipher

Constant-time crypto wrappers for ESP32 hardware accelerators.

## Overview

`tpt-e-cipher` provides trait-based abstractions over ESP32 crypto
peripherals (AES, SHA-256, ECC). The API guarantees constant-time
execution — no data-dependent branching or timing variations.

## Key Types

- `AesEngine` / `MockAesEngine` — constant-time, algebraic GF(2^8) AES-128
  (the "mock" here means software rather than hardware-peripheral-backed,
  not a fake/XOR placeholder — both are the same real, tested algorithm)
- `Sha256Engine` / `MockSha256Engine` — real, incremental SHA-256 (software;
  no hardware peripheral backend exists yet)
- `P256Ecc` / `MockP256Ecc` — P-256 ECDSA sign/verify (software; likewise no
  hardware backend yet)

## Traits

- `Aes` — Constant-time AES block encryption
- `Sha256` — Incremental SHA-256 hashing
- `Ecc` — ECDSA keygen/sign/verify

## Status

| Component | Status |
|-----------|--------|
| SHA-256 | Real implementation, verified against NIST KATs |
| AES-128 | Real, constant-time (algebraic S-box), verified against FIPS-197/SP 800-38A/CAVP vectors |
| ECC (P-256 ECDSA) | Real implementation (FIPS 186-4), cross-checked against an independent implementation (.NET CNG). Point arithmetic is branch-free on point structure/scalar bits (complete addition formulas); the underlying field arithmetic is not yet constant-time — see `ecc.rs`'s module docs |

No component here has a hardware peripheral backend yet — all three are
software implementations, gated behind `mock` only in the sense of "runs on
host without ESP32 hardware," not "fake logic."

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

AES-128 and SHA-256 are algorithmically constant-time (fixed operation
count, no data-dependent branches) but have not undergone hardware timing
measurement. ECDSA's point arithmetic is branch-free on secret data, but
the underlying big-integer field arithmetic still uses value-dependent
branches (see `ecc.rs`'s module docs for the exact boundary) — do not use
ECDSA signing here where timing side channels are in the threat model.
