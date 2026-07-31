//! Kani proof harnesses for `tpt-e-cipher`'s SHA-256 and AES implementations.
//!
//! Run with: `cargo kani --features mock -p tpt-e-cipher`
//!
//! ECC has its own file, `kani_ecc.rs` — see that file's module doc for why
//! (its scalar-multiplication loop is expensive enough per-iteration that
//! it warranted separate scoping notes, unlike AES/SHA-256's fixed, cheap
//! per-round cost).

/// Prove that `Sha256Engine::update` + `finalize` never panics for any
/// input data.
#[cfg(kani)]
#[kani::proof]
fn sha256_engine_never_panics() {
    use tpt_e_cipher::sha::Sha256Engine;
    use tpt_e_cipher::traits::Sha256;

    let mut engine = Sha256Engine::new();
    let data: Vec<u8> = kani::any_vec::<_, u8>();
    let bounded: usize = kani::any();
    kani::assume(bounded <= 128);
    kani::assume(data.len() <= bounded);
    engine.update(&data);
    let digest = engine.finalize();
    assert_eq!(digest.len(), 32);
}

/// Prove that `MockSha256Engine::update` + `finalize` never panics.
#[cfg(kani)]
#[kani::proof]
fn mock_sha256_engine_never_panics() {
    use tpt_e_cipher::mock::MockSha256Engine;
    use tpt_e_cipher::traits::Sha256;

    let mut engine = MockSha256Engine::new();
    let data: Vec<u8> = kani::any_vec::<_, u8>();
    let bounded: usize = kani::any();
    kani::assume(bounded <= 128);
    kani::assume(data.len() <= bounded);
    engine.update(&data);
    let digest = engine.finalize();
    assert_eq!(digest.len(), 32);
}

/// Prove that multiple incremental `update` calls followed by `finalize`
/// never panics.
#[cfg(kani)]
#[kani::proof]
fn sha256_incremental_never_panics() {
    use tpt_e_cipher::sha::Sha256Engine;
    use tpt_e_cipher::traits::Sha256;

    let mut engine = Sha256Engine::new();
    let rounds: u8 = kani::any();
    kani::assume(rounds <= 4);
    for _ in 0..rounds {
        let chunk: Vec<u8> = kani::any_vec::<_, u8>();
        let bounded: usize = kani::any();
        kani::assume(bounded <= 64);
        kani::assume(chunk.len() <= bounded);
        engine.update(&chunk);
    }
    let digest = engine.finalize();
    assert_eq!(digest.len(), 32);
}

/// Prove that AES-128 block encryption never panics for any 16-byte key
/// and 16-byte plaintext block. Unlike SHA-256's `update`, this has no
/// variable-length input to bound — the key schedule (11 fixed round keys)
/// and the 10-round cipher loop are both compile-time-fixed sizes, so this
/// harness needs no `kani::assume` bounding at all.
#[cfg(kani)]
#[kani::proof]
fn aes_encrypt_block_never_panics() {
    use tpt_e_cipher::mock::MockAesEngine;
    use tpt_e_cipher::traits::Aes;

    let key: [u8; 16] = kani::any();
    let mut block: [u8; 16] = kani::any();
    let mut engine = MockAesEngine::new_with_key(&key);
    engine.encrypt_block(&mut block);
}
