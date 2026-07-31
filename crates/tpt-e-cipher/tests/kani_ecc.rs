//! Kani proof harnesses for `tpt-e-cipher`'s P-256 ECDSA (`P256Ecc`).
//!
//! Run with: `cargo kani --features mock -p tpt-e-cipher`
//!
//! Scope note: `point_mul`'s inner loop is a compile-time-fixed 256
//! iterations (4 limbs × 64 bits, not a symbolic/unbounded count), so Kani
//! can fully unwind it — unlike `sign()`, which wraps `point_mul` in a
//! `loop { ... continue ... }` retry (extremely rare in practice, but not a
//! statically-bounded iteration count), making it a much harder Kani
//! target. This file therefore covers `keygen` and `verify` (each a single,
//! bounded pass through the real 256-iteration scalar multiplication) and
//! deliberately does not attempt `sign()`. As with every other crypto Kani
//! harness in this workspace, this proves panic-freedom over arbitrary
//! symbolic input, not functional correctness — that's covered separately
//! by `tests/proptest_ecc.rs` (property tests) and
//! `ecc.rs::tests::ecdsa_cross_check_against_dotnet_cng` (KAT/independent
//! cross-check). Not run locally — Kani doesn't build on native Windows
//! (see the root `todo.md`); needs CI (`kani.yml`, `ubuntu-latest`) to
//! confirm both proofs actually terminate in reasonable time, given the
//! real field-arithmetic cost per loop iteration.

/// `keygen` never panics for an arbitrary 32-byte seed.
#[cfg(kani)]
#[kani::proof]
fn ecc_keygen_never_panics() {
    use tpt_e_cipher::ecc::P256Ecc;
    use tpt_e_cipher::traits::Ecc;

    let seed: [u8; 32] = kani::any();
    let ecc = P256Ecc;
    let _ = ecc.keygen(&seed);
}

/// `verify` never panics for an arbitrary hash, signature, and public key
/// — including inputs that don't correspond to a real signing operation
/// (e.g. an arbitrary `(r, s)` pair against an arbitrary on-curve point).
#[cfg(kani)]
#[kani::proof]
fn ecc_verify_never_panics() {
    use tpt_e_cipher::ecc::{P256Ecc, Signature};
    use tpt_e_cipher::traits::Ecc;

    // Derive an arbitrary but genuinely on-curve public key via keygen,
    // rather than arbitrary (x, y) coordinates — `PublicKey::from_xy`
    // already rejects off-curve points before `verify` ever sees them
    // (proven separately by its own logic), so an off-curve point here
    // would only exercise that earlier rejection, not `verify` itself.
    let seed: [u8; 32] = kani::any();
    let ecc = P256Ecc;
    let (pk, _) = ecc.keygen(&seed);

    let r: [u8; 32] = kani::any();
    let s: [u8; 32] = kani::any();
    let hash: [u8; 32] = kani::any();
    let sig = Signature::from_rs(&r, &s);

    let _ = ecc.verify(&hash, &sig, &pk);
}
