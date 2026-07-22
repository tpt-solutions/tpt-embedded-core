//! Property tests for `MockSha256Engine`.
//!
//! These exercise the incremental-hashing correctness property that the
//! old, buffer-capped mock did not have: splitting input across any number
//! of `update` calls must produce the same digest as one bulk update, and
//! there must be no artificial length cap.

#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_cipher::traits::Sha256;

fn digest_bulk(data: &[u8]) -> [u8; 32] {
    let mut engine = tpt_e_cipher::mock::MockSha256Engine::new();
    engine.update(data);
    engine.finalize()
}

fn digest_chunked(data: &[u8], chunk_sizes: &[usize]) -> [u8; 32] {
    let mut engine = tpt_e_cipher::mock::MockSha256Engine::new();
    let mut offset = 0;
    for &size in chunk_sizes {
        if offset >= data.len() {
            break;
        }
        let end = (offset + size).min(data.len());
        engine.update(&data[offset..end]);
        offset = end;
    }
    if offset < data.len() {
        engine.update(&data[offset..]);
    }
    engine.finalize()
}

proptest! {
    /// Chunking input arbitrarily (including chunks smaller than a SHA-256
    /// block, and total input well over the old 256-byte mock cap) must
    /// never change the resulting digest.
    #[test]
    fn chunked_update_matches_bulk_update(
        data in proptest::collection::vec(any::<u8>(), 0..600),
        chunk_sizes in proptest::collection::vec(1usize..17, 1..40),
    ) {
        let bulk = digest_bulk(&data);
        let chunked = digest_chunked(&data, &chunk_sizes);
        prop_assert_eq!(bulk, chunked);
    }

    /// Hashing never panics for any input length, including well past the
    /// old mock's 256-byte cap.
    #[test]
    fn never_panics_for_any_length(data in proptest::collection::vec(any::<u8>(), 0..2000)) {
        let mut engine = tpt_e_cipher::mock::MockSha256Engine::new();
        engine.update(&data);
        let _ = engine.finalize();
    }

    /// Digest is a deterministic function of the input.
    #[test]
    fn deterministic(data in proptest::collection::vec(any::<u8>(), 0..300)) {
        prop_assert_eq!(digest_bulk(&data), digest_bulk(&data));
    }
}
