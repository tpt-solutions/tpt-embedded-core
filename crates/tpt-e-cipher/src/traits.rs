//! Trait definitions for constant-time crypto operations.

/// An AES encryption engine with constant-time guarantees.
pub trait Aes {
    /// Encrypt a single 16-byte block in-place.
    fn encrypt_block(&mut self, block: &mut [u8; 16]);
}

/// A SHA-256 hashing engine.
pub trait Sha256 {
    /// Feed data into the hash.
    fn update(&mut self, data: &[u8]);
    /// Produce the final digest.
    fn finalize(self) -> [u8; 32];
}

/// Elliptic-curve cryptography operations.
///
/// Provides an abstract interface over curve arithmetic, allowing the same
/// API to be backed by hardware crypto peripherals (via `esp-hal`) or by
/// a software implementation for host-side testing.
///
/// The associated types are opaque — callers interact with them only through
/// the trait methods. This allows the backend to choose its own internal
/// representation (e.g., affine vs. Jacobian coordinates for points).
pub trait Ecc: core::fmt::Debug {
    /// An ECDSA public key.
    type PublicKey: core::fmt::Debug;
    /// An ECDSA private (secret) key.
    type PrivateKey: core::fmt::Debug;
    /// An ECDSA signature (r, s).
    type Signature: core::fmt::Debug;

    /// Generate a new random key pair.
    fn keygen(&self) -> (Self::PublicKey, Self::PrivateKey);

    /// Sign a32-byte message hash using ECDSA.
    ///
    /// # Security
    ///
    /// The implementation MUST use a cryptographically secure source of
    /// ephemeral randomness (nonce `k`). Reusing or biasing `k` leaks the
    /// private key.
    fn sign(&self, hash: &[u8; 32], private_key: &Self::PrivateKey) -> Self::Signature;

    /// Verify an ECDSA signature against a message hash and public key.
    ///
    /// Returns `true` if and only if the signature is valid.
    fn verify(
        &self,
        hash: &[u8; 32],
        signature: &Self::Signature,
        public_key: &Self::PublicKey,
    ) -> bool;
}
