//! AES block cipher wrapper.

use crate::traits::Aes;

/// Placeholder for the AES peripheral wrapper.
#[derive(Debug, Copy, Clone)]
pub struct AesEngine;

impl Aes for AesEngine {
    fn encrypt_block(&mut self, _block: &mut [u8; 16]) {
        // Hardware peripheral access will be implemented in Phase 3.
    }
}
