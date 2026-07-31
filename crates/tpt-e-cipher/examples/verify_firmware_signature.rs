//! Signed firmware verification — the pattern this crate's real, working
//! P-256 ECDSA + SHA-256 exist to enable, but that nothing else in this
//! repo demonstrates end to end.
//!
//! Sketch: a build server hashes a firmware image and signs the hash with
//! its P-256 private key. The device only has the corresponding *public*
//! key baked in. Before "installing" a new image, the device hashes what
//! it received and checks the signature against its known public key — an
//! attacker who can modify the image in transit (or on a compromised
//! update server) cannot produce a signature that verifies, because they
//! don't have the private key.
//!
//! This example plays both roles (build server + device) in one process,
//! using only `tpt-e-cipher`'s existing `mock` feature (`MockSha256Engine`,
//! `MockP256Ecc`) — no new library API, purely a composition of what's
//! already proven by this crate's own test suite. Real firmware would keep
//! the private key off-device entirely (only the public key ships in the
//! binary) and would source `seed`/nonce randomness from a real CSPRNG
//! rather than a fixed byte pattern.
//!
//! Run with:
//!
//! ```text
//! cargo run -p tpt-e-cipher --example verify_firmware_signature --features mock
//! ```

use tpt_e_cipher::mock::{MockP256Ecc, MockSha256Engine};
use tpt_e_cipher::traits::{Ecc, Sha256};

/// Hash a firmware image with SHA-256.
fn hash_firmware(image: &[u8]) -> [u8; 32] {
    let mut hasher = MockSha256Engine::new();
    hasher.update(image);
    hasher.finalize()
}

fn main() {
    let ecc = MockP256Ecc;

    // --- Build server: generate a signing keypair, sign a firmware image. ---
    // A real build server sources this seed from a CSPRNG and never lets
    // the private key leave its signing infrastructure.
    let build_server_seed = [0x42u8; 32];
    let (public_key, private_key) = ecc.keygen(&build_server_seed);

    let firmware_v1 = b"tpt-embedded-core firmware v1.0.0 :: (imagine real machine code here)";
    let firmware_hash = hash_firmware(firmware_v1);
    let signature = ecc.sign(&firmware_hash, &private_key);

    println!("Build server: signed a {}-byte firmware image.", firmware_v1.len());

    // --- Device: only ever sees `public_key` (baked in at manufacture
    // time) plus whatever image + signature arrive over the update channel.
    let accept_update = |image: &[u8], sig: &<MockP256Ecc as Ecc>::Signature| -> bool {
        let hash = hash_firmware(image);
        ecc.verify(&hash, sig, &public_key)
    };

    // Case 1: genuine image, untouched in transit — accepted.
    let genuine_ok = accept_update(firmware_v1, &signature);
    println!(
        "Device: genuine image + valid signature -> {}",
        if genuine_ok { "ACCEPTED" } else { "REJECTED" }
    );
    assert!(genuine_ok, "a genuinely signed image must be accepted");

    // Case 2: the same signature, but the image was tampered with after
    // signing (a single flipped byte) — the hash no longer matches what
    // was signed, so verification fails and the device must refuse to
    // install it.
    let mut tampered = firmware_v1.to_vec();
    tampered[10] ^= 0x01;
    let tampered_ok = accept_update(&tampered, &signature);
    println!(
        "Device: tampered image + same signature   -> {}",
        if tampered_ok { "ACCEPTED" } else { "REJECTED" }
    );
    assert!(!tampered_ok, "a tampered image must be rejected");

    // Case 3: a genuine-looking image signed by someone *without* the real
    // private key (e.g. an attacker's own keypair) — also rejected, since
    // the device only trusts signatures verifying under its baked-in
    // `public_key`.
    let attacker_seed = [0x99u8; 32];
    let (_attacker_pk, attacker_sk) = ecc.keygen(&attacker_seed);
    let forged_signature = ecc.sign(&firmware_hash, &attacker_sk);
    let forged_ok = accept_update(firmware_v1, &forged_signature);
    println!(
        "Device: genuine image + attacker signature -> {}",
        if forged_ok { "ACCEPTED" } else { "REJECTED" }
    );
    assert!(!forged_ok, "a signature from an untrusted key must be rejected");

    println!("\nAll three cases behaved correctly: signed-and-untampered accepted, tampered or unauthorized rejected.");
}
