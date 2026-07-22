//! Software crypto backend for host-side testing.
//!
//! **Note**: This mock is NOT constant-time. It is for logic testing only.
//! Timing analysis must be performed against the hardware implementation.

use crate::sha256_core::Sha256Core;
use crate::traits::{Aes, Sha256};

/// Mock AES engine using a simplified XOR-based cipher.
///
/// **WARNING**: This implementation is NOT constant-time. It uses data-dependent
/// operations which are vulnerable to timing side-channel attacks. This is
/// acceptable for host-side logic testing only.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct MockAesEngine {
    round_keys: [[u8; 16]; 11],
}

impl MockAesEngine {
    /// Create a new mock AES engine with a fixed key for testing.
    pub fn new() -> Self {
        let mut engine = Self {
            round_keys: [[0u8; 16]; 11],
        };
        let test_key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
            0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
        ];
        engine.round_keys[0] = test_key;
        engine
    }
}

impl Default for MockAesEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Aes for MockAesEngine {
    fn encrypt_block(&mut self, block: &mut [u8; 16]) {
        for (b, k) in block.iter_mut().zip(self.round_keys[0].iter()) {
            *b ^= k;
        }
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
