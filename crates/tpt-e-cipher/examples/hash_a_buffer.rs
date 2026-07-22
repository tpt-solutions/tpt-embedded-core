//! Minimal example: hash a buffer with the software SHA-256 engine.
//!
//! Run with:
//!
//! ```text
//! cargo run -p tpt-e-cipher --example hash_a_buffer --features mock
//! ```

use tpt_e_cipher::mock::MockSha256Engine;
use tpt_e_cipher::traits::Sha256;

fn main() {
    let mut engine = MockSha256Engine::new();
    engine.update(b"hello, ");
    engine.update(b"tpt-embedded-core!");
    let digest = engine.finalize();

    print!("SHA-256: ");
    for byte in digest {
        print!("{byte:02x}");
    }
    println!();
}
