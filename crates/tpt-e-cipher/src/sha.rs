//! SHA-256 hashing wrapper.

use crate::traits::Sha256;

/// Placeholder for the SHA-256 peripheral wrapper.
#[derive(Debug, Copy, Clone)]
pub struct Sha256Engine;

impl Sha256 for Sha256Engine {
    fn update(&mut self, _data: &[u8]) {}
    fn finalize(self) -> [u8; 32] {
        [0u8; 32]
    }
}
