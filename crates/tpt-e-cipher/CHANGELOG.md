# Changelog — `tpt-e-cipher`

Crates in this workspace are currently version-synchronized (see the root
`README.md`'s "Versioning & Publishing" section); this per-crate log tracks
notable changes scoped to this crate specifically. See the workspace root
`CHANGELOG.md` for the cross-crate view, and `todo.md` for the full,
dated audit trail this summarizes.

## Unreleased

### Added

- Real AES-128 (constant-time, algebraic GF(2^8) S-box) and P-256 ECDSA
  implementations, replacing earlier stubs. Both verified against NIST/
  FIPS/CAVP known-answer vectors; ECDSA additionally cross-checked against
  an independent implementation (.NET's `ECDsa`/CNG).
- `point_add`/`point_double` rewritten using the Renes–Costello–Batina
  complete addition/doubling formulas (cross-checked against RustCrypto's
  `primeorder` crate), removing the previous branching on point structure
  (identity/doubling/inverse). Scalar-bit accumulator selection is now a
  constant-time bitmask select.
- Proptest coverage for AES (`encrypt_block`) and ECC (sign/verify
  round-trip, wrong-hash/wrong-key rejection, determinism), and Kani
  panic-freedom harnesses for AES's block cipher and ECC's `keygen`/`verify`.
- `MockP256Ecc`, mirroring the `Mock*Engine` pattern already used by AES/SHA-256.

### Fixed

- `keygen()` previously returned a hardcoded private-key scalar; now
  derives from a caller-supplied CSPRNG seed.
- `verify()` didn't validate that the public key lies on the curve — an
  invalid-curve attack surface for untrusted key bytes.
- Neither `sign()` nor `verify()` reduced the message hash mod n (FIPS
  186-4 requirement); signatures were malleable (no canonical low-s
  enforcement). Both fixed.
- `point_mul`'s operation count depended on the secret scalar's bit
  length — a timing side channel. Now always performs 256 fixed
  double+select iterations.

### Known limitations

- The underlying big-integer field arithmetic (`fe_add`/`fe_sub`'s
  conditional-subtract modular reduction, `fe_inv`, `u256_cmp`) still uses
  value-dependent branches and is not yet constant-time — see `ecc.rs`'s
  module docs for the precise boundary. Do not use ECDSA signing here
  where timing side channels are in the threat model.
- No hardware crypto peripheral backend exists yet for any of AES/SHA-256/
  ECC — all are software implementations today.
