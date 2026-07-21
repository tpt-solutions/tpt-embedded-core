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
