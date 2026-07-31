//! Software crypto backend for host-side testing.
//!
//! **Note**: The mock AES is NOT constant-time. It is for logic testing only.
//! Timing analysis must be performed against the hardware implementation.

use crate::ecc::{P256Ecc, PrivateKey, PublicKey, Signature};
use crate::sha256_core::Sha256Core;
use crate::traits::{Aes, Ecc, Sha256};

/// Mock AES engine for host-side logic testing.
///
/// Uses the same algebraic GF(2^8) S-box as [`crate::aes::AesEngine`]
/// (which IS constant-time) but wrapped here for backward compatibility
/// with existing test code. The mock label reflects that this is a
/// software fallback, not a hardware peripheral — the algorithm itself
/// is constant-time.
#[derive(Debug, Clone, Copy)]
pub struct MockAesEngine {
    round_keys: [[u8; 16]; 11],
}

impl MockAesEngine {
    /// Create a new mock AES engine with a fixed test key.
    pub fn new() -> Self {
        let test_key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
            0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
        ];
        Self::new_with_key(&test_key)
    }

    /// Create a new mock AES engine with a caller-supplied key.
    ///
    /// The key must be 16 bytes (AES-128). For testing with known-answer
    /// vectors, use the constants from FIPS-197 Appendix B.
    pub fn new_with_key(key: &[u8; 16]) -> Self {
        Self {
            round_keys: crate::aes::expand_key(key),
        }
    }
}

impl Default for MockAesEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Aes for MockAesEngine {
    fn encrypt_block(&mut self, block: &mut [u8; 16]) {
        crate::aes::add_round_key(block, &self.round_keys[0]);
        for round in 1..10 {
            for byte in block.iter_mut() {
                *byte = crate::aes::aes_sbox(*byte);
            }
            crate::aes::shift_rows(block);
            crate::aes::mix_columns(block);
            crate::aes::add_round_key(block, &self.round_keys[round]);
        }
        for byte in block.iter_mut() {
            *byte = crate::aes::aes_sbox(*byte);
        }
        crate::aes::shift_rows(block);
        crate::aes::add_round_key(block, &self.round_keys[10]);
    }
}

/// Mock SHA-256 engine.
///
/// This is a genuinely correct SHA-256 implementation (see
/// [`crate::sha256_core::Sha256Core`], the same core `sha::Sha256Engine`
/// uses) — it accepts input of any length with no artificial buffer cap,
/// and chunked updates produce the same digest as a single bulk update.
/// It is labeled "mock" only because it is a software fallback for
/// host-side testing, not a hardware implementation with a proven-constant-
/// time execution path.
#[derive(Debug, Copy, Clone, Default)]
pub struct MockSha256Engine {
    core: Sha256Core,
}

impl MockSha256Engine {
    /// Create a new mock SHA-256 engine.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Sha256 for MockSha256Engine {
    fn update(&mut self, data: &[u8]) {
        self.core.update(data);
    }

    fn finalize(self) -> [u8; 32] {
        self.core.finalize()
    }
}

/// Mock P-256 ECDSA engine for host-side testing.
///
/// Thin delegating wrapper around [`crate::ecc::P256Ecc`] — added to mirror
/// the `Engine`/`Mock*Engine` split used for AES and SHA-256. Unlike those
/// two, there is not yet a separate hardware-backed `Ecc` implementation,
/// so `MockP256Ecc` and `P256Ecc` are behaviorally identical (both software,
/// both not constant-time — see `crate::ecc`'s module docs). This exists
/// purely so callers that generically pick a "mock" backend by name have a
/// symmetrical `MockP256Ecc` alongside `MockAesEngine`/`MockSha256Engine`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockP256Ecc;

impl Ecc for MockP256Ecc {
    type PublicKey = PublicKey;
    type PrivateKey = PrivateKey;
    type Signature = Signature;

    fn keygen(&self, seed: &[u8; 32]) -> (PublicKey, PrivateKey) {
        P256Ecc.keygen(seed)
    }

    fn sign(&self, hash: &[u8; 32], private_key: &PrivateKey) -> Signature {
        P256Ecc.sign(hash, private_key)
    }

    fn verify(&self, hash: &[u8; 32], signature: &Signature, public_key: &PublicKey) -> bool {
        P256Ecc.verify(hash, signature, public_key)
    }
}
