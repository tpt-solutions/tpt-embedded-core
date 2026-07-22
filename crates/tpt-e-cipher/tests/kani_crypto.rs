//! Kani proof harnesses for `tpt-e-cipher` SHA-256 implementation.
//!
//! Run with: `cargo kani --features mock -p tpt-e-cipher`

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
