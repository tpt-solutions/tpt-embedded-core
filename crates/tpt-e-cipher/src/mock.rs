//! Software crypto backend for host-side testing.
//!
//! **Note**: This mock is NOT constant-time. It is for logic testing only.
//! Timing analysis must be performed against the hardware implementation.

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

/// Mock SHA-256 engine using a simplified hash.
///
/// **WARNING**: This implementation is NOT constant-time. It is for logic
/// testing only. Accepts up to 256 bytes of input.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct MockSha256Engine {
    state: [u32; 8],
    buffer: [u8; 256],
    buffer_len: usize,
}

impl MockSha256Engine {
    /// Create a new mock SHA-256 engine.
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: [0u8; 256],
            buffer_len: 0,
        }
    }
}

impl Default for MockSha256Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 for MockSha256Engine {
    fn update(&mut self, data: &[u8]) {
        let remaining = 256 - self.buffer_len;
        let copy_len = if data.len() < remaining { data.len() } else { remaining };
        self.buffer[self.buffer_len..self.buffer_len + copy_len]
            .copy_from_slice(&data[..copy_len]);
        self.buffer_len += copy_len;
    }

    fn finalize(mut self) -> [u8; 32] {
        for i in 0..self.buffer_len {
            let word_idx = i % 8;
            self.state[word_idx] = self.state[word_idx]
                .wrapping_add(self.buffer[i] as u32)
                .rotate_left(3);
        }

        let mut result = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            result[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        result
    }
}
