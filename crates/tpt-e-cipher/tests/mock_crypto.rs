//! Tests for mock crypto implementations.

#![allow(missing_docs)]

use tpt_e_cipher::traits::{Aes, Ecc, Sha256};

/// Test that mock AES engine can encrypt a block.
#[test]
fn mock_aes_encrypt_block() {
    let mut engine = tpt_e_cipher::mock::MockAesEngine::new();
    let mut block = [0u8; 16];
    engine.encrypt_block(&mut block);
    // The mock XORs with round key, so block should be non-zero
    // (unless the round key is all zeros, which it's not for our test key)
    assert_ne!(block, [0u8; 16]);
}

/// Test that mock AES encryption is deterministic.
#[test]
fn mock_aes_deterministic() {
    let mut engine1 = tpt_e_cipher::mock::MockAesEngine::new();
    let mut engine2 = tpt_e_cipher::mock::MockAesEngine::new();

    let mut block1 = [1u8; 16];
    let mut block2 = [1u8; 16];

    engine1.encrypt_block(&mut block1);
    engine2.encrypt_block(&mut block2);

    assert_eq!(block1, block2);
}

/// `MockP256Ecc` must round-trip sign/verify identically to `P256Ecc`,
/// proving it's a genuine (if delegating) `Ecc` implementation and not
/// just a type alias that happens to compile.
#[test]
fn mock_ecc_sign_verify_round_trip() {
    let ecc = tpt_e_cipher::mock::MockP256Ecc;
    let seed = [7u8; 32];
    let (pk, sk) = ecc.keygen(&seed);
    let hash = [0x99u8; 32];
    let sig = ecc.sign(&hash, &sk);
    assert!(ecc.verify(&hash, &sig, &pk), "MockP256Ecc round-trip must verify");
}

/// Test that mock AES encryption changes the block.
#[test]
fn mock_aes_changes_block() {
    let mut engine = tpt_e_cipher::mock::MockAesEngine::new();
    let original = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A,
                    0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55];
    let mut block = original;
    engine.encrypt_block(&mut block);
    // With our test key, XOR should change the block
    // (unless the round key equals the block, which is unlikely)
    assert_ne!(block, original);
}

/// Test that mock SHA-256 engine can be created and used.
#[test]
fn mock_sha256_basic() {
    let mut engine = tpt_e_cipher::mock::MockSha256Engine::new();
    engine.update(b"hello");
    let hash = engine.finalize();
    // Hash should be 32 bytes
    assert_eq!(hash.len(), 32);
}

/// Test that mock SHA-256 is deterministic.
#[test]
fn mock_sha256_deterministic() {
    let mut engine1 = tpt_e_cipher::mock::MockSha256Engine::new();
    let mut engine2 = tpt_e_cipher::mock::MockSha256Engine::new();

    engine1.update(b"test data");
    engine2.update(b"test data");

    let hash1 = engine1.finalize();
    let hash2 = engine2.finalize();

    assert_eq!(hash1, hash2);
}

/// Test that mock SHA-256 produces different hashes for different inputs.
#[test]
fn mock_sha256_different_inputs() {
    let mut engine1 = tpt_e_cipher::mock::MockSha256Engine::new();
    let mut engine2 = tpt_e_cipher::mock::MockSha256Engine::new();

    engine1.update(b"hello");
    engine2.update(b"world");

    let hash1 = engine1.finalize();
    let hash2 = engine2.finalize();

    assert_ne!(hash1, hash2);
}

/// Test that mock SHA-256 handles empty input.
#[test]
fn mock_sha256_empty() {
    let engine = tpt_e_cipher::mock::MockSha256Engine::new();
    let hash = engine.finalize();
    assert_eq!(hash.len(), 32);
}

/// Test that mock SHA-256 handles multiple updates identically to a single
/// bulk update — this is the actual incremental-hashing correctness
/// property SHA-256 is required to have.
#[test]
fn mock_sha256_multiple_updates() {
    let mut engine1 = tpt_e_cipher::mock::MockSha256Engine::new();
    let mut engine2 = tpt_e_cipher::mock::MockSha256Engine::new();

    // Update in parts
    engine1.update(b"hel");
    engine1.update(b"lo");

    // Update all at once
    engine2.update(b"hello");

    let hash1 = engine1.finalize();
    let hash2 = engine2.finalize();

    assert_eq!(hash1, hash2);
}

/// Test that input longer than the old (removed) 256-byte mock buffer cap
/// is handled correctly instead of being silently truncated.
#[test]
fn mock_sha256_handles_input_over_256_bytes() {
    let mut engine = tpt_e_cipher::mock::MockSha256Engine::new();
    let data = [0x5Au8; 1000];
    engine.update(&data);
    let hash = engine.finalize();
    assert_eq!(hash.len(), 32);

    // Splitting the same input across many small updates must still yield
    // the same digest as one bulk update.
    let mut chunked = tpt_e_cipher::mock::MockSha256Engine::new();
    for chunk in data.chunks(7) {
        chunked.update(chunk);
    }
    assert_eq!(chunked.finalize(), hash);
}

/// NIST/FIPS known-answer tests for SHA-256. Vectors independently
/// verified with `sha256sum` at authoring time.
mod sha256_kats {
    use tpt_e_cipher::traits::Sha256;

    fn digest_hex(data: &[u8]) -> String {
        let mut engine = tpt_e_cipher::mock::MockSha256Engine::new();
        engine.update(data);
        engine
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// SHA-256("") — NIST test vector.
    #[test]
    fn empty_string() {
        assert_eq!(
            digest_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// SHA-256("abc") — NIST/FIPS 180-4 test vector.
    #[test]
    fn abc() {
        assert_eq!(
            digest_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// SHA-256 of the 448-bit NIST multi-block test vector.
    #[test]
    fn two_block_message() {
        assert_eq!(
            digest_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }
}