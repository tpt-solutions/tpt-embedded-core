# `tpt-e-cipher`

Formally verified, constant-time HW crypto wrappers (AES, SHA-256, ECC) for
ESP32.

Part of [`tpt-embedded-core`](https://github.com/tpt-solutions/tpt-embedded-core),
a proof-native `no_std` foundation for ESP32 ecosystems.

## What it does

Trait-based abstractions (`Aes`, `Sha256`, `Ecc`) over crypto operations,
currently backed by software implementations (no hardware peripheral
backend exists yet):

- **AES-128** — constant-time, algebraic GF(2^8) S-box, verified against
  FIPS-197/SP 800-38A/CAVP known-answer vectors
- **SHA-256** — real, incremental, FIPS 180-4 implementation, verified
  against NIST KATs
- **P-256 ECDSA** — sign/verify per FIPS 186-4 (hash-to-scalar reduction,
  canonical low-s signatures, public-key on-curve validation), cross-checked
  against an independent implementation (.NET's `ECDsa`/CNG). Point
  arithmetic uses the Renes–Costello–Batina complete addition/doubling
  formulas plus a bitmask-based scalar-bit select, so point structure
  (identity/doubling/inverse) and the scalar's bit pattern don't drive
  control flow. The underlying big-integer field arithmetic (modular
  reduction, inversion) is not yet constant-time — see `ecc.rs`'s module
  docs for the precise boundary before using this where signing-timing side
  channels are in the threat model.

## Example

```rust
use tpt_e_cipher::sha::Sha256Engine;
use tpt_e_cipher::traits::Sha256;

let mut hasher = Sha256Engine::new();
hasher.update(b"hello");
let digest = hasher.finalize();
assert_eq!(digest.len(), 32);
```

Run it: `cargo run -p tpt-e-cipher --example hash_a_buffer --features mock`

For a realistic end-to-end use case — signing a firmware image off-device
and verifying it on-device, rejecting both tampering and unauthorized
signers — see:
`cargo run -p tpt-e-cipher --example verify_firmware_signature --features mock`

## License

Dual-licensed under MIT OR Apache-2.0. See the
[repository root](https://github.com/tpt-solutions/tpt-embedded-core) for
full docs, architecture, and the other four crates.
