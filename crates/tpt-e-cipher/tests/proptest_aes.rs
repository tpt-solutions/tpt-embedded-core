//! Property tests for `MockAesEngine`.
//!
//! These exercise the algebraic AES-128 implementation in `aes.rs` through
//! the `MockAesEngine` wrapper: never-panic under arbitrary input,
//! deterministic output, and FIPS-197 Appendix B known-answer correctness.

#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_cipher::mock::MockAesEngine;
use tpt_e_cipher::traits::Aes;

proptest! {
    /// Encrypting an arbitrary 16-byte block must never panic, regardless
    /// of the byte values.
    #[test]
    fn encrypt_block_never_panics(block in [any::<u8>(); 16]) {
        let mut engine = MockAesEngine::new();
        let mut b = block;
        engine.encrypt_block(&mut b);
    }

    /// Encryption is deterministic: encrypting the same block twice through
    /// two independent engine instances must produce identical output.
    #[test]
    fn encrypt_block_is_deterministic(block in [any::<u8>(); 16]) {
        let mut engine1 = MockAesEngine::new();
        let mut engine2 = MockAesEngine::new();

        let mut b1 = block;
        let mut b2 = block;

        engine1.encrypt_block(&mut b1);
        engine2.encrypt_block(&mut b2);

        prop_assert_eq!(b1, b2);
    }

    /// The mock engine must match the FIPS-197 Appendix B known-answer
    /// vector: encrypting the standard test plaintext with the standard
    /// test key must always produce the standard test ciphertext.
    #[test]
    fn fips_197_appendix_b_vector(_seed in any::<u64>()) {
        // Fixed constants from FIPS-197 Appendix B — same values used in
        // `tests/mock_crypto.rs::mock_aes_encrypt_block` and the hardware
        // smoke tests in `firmware/`.
        const KEY: [u8; 16] = [
            0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
            0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C,
        ];
        const PLAINTEXT: [u8; 16] = [
            0x32, 0x43, 0xF6, 0xA8, 0x88, 0x5A, 0x30, 0x8D,
            0x31, 0x31, 0x98, 0xA2, 0xE0, 0x37, 0x07, 0x34,
        ];
        const EXPECTED_CIPHERTEXT: [u8; 16] = [
            0x39, 0x25, 0x84, 0x1D, 0x02, 0xDC, 0x09, 0xFB,
            0xDC, 0x11, 0x85, 0x97, 0x19, 0x6A, 0x0B, 0x32,
        ];

        let mut engine = MockAesEngine::new_with_key(&KEY);
        let mut ciphertext = PLAINTEXT;
        engine.encrypt_block(&mut ciphertext);

        prop_assert_eq!(ciphertext, EXPECTED_CIPHERTEXT);
    }
}
