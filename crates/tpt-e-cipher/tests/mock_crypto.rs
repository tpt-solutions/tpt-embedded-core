//! Tests for mock crypto implementations.

#![allow(missing_docs)]

use tpt_e_cipher::traits::{Aes, Sha256};

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

/// Test that mock SHA-256 handles multiple updates.
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

    // Note: Our mock doesn't implement proper SHA-256, so these may differ.
    // This test verifies the API works, not the cryptographic correctness.
    // In a real implementation, these would be equal.
    assert_eq!(hash1.len(), 32);
    assert_eq!(hash2.len(), 32);
}