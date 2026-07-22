//! SHA-256 hashing wrapper.
//!
//! Backed by a correct, pure-Rust software SHA-256 implementation
//! ([`crate::sha256_core::Sha256Core`]) pending real `esp-hal` crypto
//! peripheral wiring. This is functionally correct (verified against the
//! NIST/FIPS known-answer tests in `tests/mock_crypto.rs`), but has not yet
//! been proven constant-time — see the crate's outstanding formal
//! verification item in `todo.md`.

use crate::sha256_core::Sha256Core;
use crate::traits::Sha256;

/// Software SHA-256 engine, pending hardware peripheral wiring.
#[derive(Debug, Copy, Clone, Default)]
pub struct Sha256Engine {
    core: Sha256Core,
}

impl Sha256Engine {
    /// Create a new SHA-256 engine.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Sha256 for Sha256Engine {
    fn update(&mut self, data: &[u8]) {
        self.core.update(data);
    }

    fn finalize(self) -> [u8; 32] {
        self.core.finalize()
    }
}
