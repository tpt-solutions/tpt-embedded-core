//! Property tests for P-256 ECDSA (`P256Ecc`).
//!
//! Closes a gap flagged in the root `todo.md`: ECC previously had only
//! plain `#[test]` unit tests (KATs, a couple of hand-written round-trips),
//! no proptest coverage exercising randomized seeds/hashes the way AES and
//! SHA-256 already do.

#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_cipher::ecc::P256Ecc;
use tpt_e_cipher::traits::Ecc;

proptest! {
    /// keygen/sign/verify never panics for arbitrary seed/hash bytes, and a
    /// signature produced for a given seed+hash always verifies against the
    /// public key derived from the same seed.
    #[test]
    fn sign_verify_round_trip(seed in [any::<u8>(); 32], hash in [any::<u8>(); 32]) {
        let ecc = P256Ecc;
        let (pk, sk) = ecc.keygen(&seed);
        let sig = ecc.sign(&hash, &sk);
        prop_assert!(ecc.verify(&hash, &sig, &pk));
    }

    /// A signature produced for one hash must not verify against a
    /// different hash, for arbitrary seed/hash pairs.
    #[test]
    fn wrong_hash_fails(seed in [any::<u8>(); 32], hash in [any::<u8>(); 32], other_hash in [any::<u8>(); 32]) {
        prop_assume!(hash != other_hash);
        let ecc = P256Ecc;
        let (pk, sk) = ecc.keygen(&seed);
        let sig = ecc.sign(&hash, &sk);
        prop_assert!(!ecc.verify(&other_hash, &sig, &pk));
    }

    /// A signature produced under one seed's key must not verify against a
    /// different seed's public key, for arbitrary distinct seed pairs.
    #[test]
    fn wrong_key_fails(seed1 in [any::<u8>(); 32], seed2 in [any::<u8>(); 32], hash in [any::<u8>(); 32]) {
        prop_assume!(seed1 != seed2);
        let ecc = P256Ecc;
        let (_, sk1) = ecc.keygen(&seed1);
        let (pk2, _) = ecc.keygen(&seed2);
        let sig = ecc.sign(&hash, &sk1);
        prop_assert!(!ecc.verify(&hash, &sig, &pk2));
    }

    /// Signing the same hash with the same key twice is deterministic
    /// (the mock nonce generator is seeded from the hash, not randomness).
    #[test]
    fn sign_is_deterministic(seed in [any::<u8>(); 32], hash in [any::<u8>(); 32]) {
        let ecc = P256Ecc;
        let (_, sk) = ecc.keygen(&seed);
        let sig1 = ecc.sign(&hash, &sk);
        let sig2 = ecc.sign(&hash, &sk);
        prop_assert_eq!(sig1, sig2);
    }
}
