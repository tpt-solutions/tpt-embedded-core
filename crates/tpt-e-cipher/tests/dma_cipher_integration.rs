//! Phase 3 exit-criteria integration test: a `tpt-e-typestate-hal` DMA
//! handle feeding a `tpt-e-cipher` crypto operation.
//!
//! Drives a mock DMA channel through its full typestate chain
//! (`Idle` -> `Configured` -> `Transferring` -> `Complete`), then hashes and
//! encrypts the buffer the channel delivered — demonstrating the "DMA
//! handle -> crypto op" pipeline the two crates are meant to compose into.

#![allow(missing_docs)]
#![allow(unsafe_code)] // required to hand a `&'static mut` buffer to `DmaChannel::configure`

use tpt_e_cipher::mock::{MockAesEngine, MockSha256Engine};
use tpt_e_cipher::traits::{Aes, Sha256};
use tpt_e_typestate_hal::dma::DmaChannel;

/// Runs a 16-byte buffer through the full DMA typestate chain and returns
/// a copy of its contents once the transfer reaches `Complete`.
fn deliver_via_dma(payload: [u8; 16]) -> [u8; 16] {
    static mut DMA_BUF: [u8; 16] = [0u8; 16];

    // SAFETY: single-threaded test, no concurrent access to `DMA_BUF`.
    unsafe {
        DMA_BUF = payload;
    }

    let channel = DmaChannel::mock(0);
    // SAFETY: `DMA_BUF` is not accessed again until after `wait()` below
    // returns the channel in the `Complete` state. `&raw mut` avoids
    // creating an intermediate `&mut` to the mutable static (UB-prone per
    // the 2024 edition lint) — only the raw pointer is dereferenced, once,
    // to build the slice `configure` needs.
    let ptr: *mut u8 = (&raw mut DMA_BUF) as *mut u8;
    let buf: &'static mut [u8] = unsafe { core::slice::from_raw_parts_mut(ptr, 16) };
    let channel = channel.configure(buf, 16);
    let channel = channel.start();
    let _complete = channel.wait();

    // SAFETY: the channel is `Complete`, so the transfer has finished and
    // no further DMA access to the buffer will occur.
    unsafe { DMA_BUF }
}

#[test]
fn dma_delivered_buffer_can_be_hashed() {
    let payload = *b"hello dma cipher";

    let data = deliver_via_dma(payload);
    assert_eq!(data, payload, "mock DMA must not corrupt the buffer");

    let mut engine = MockSha256Engine::new();
    engine.update(&data);
    let digest = engine.finalize();

    // Hashing the same input directly (bypassing the DMA handoff) must
    // produce an identical digest.
    let mut direct = MockSha256Engine::new();
    direct.update(&payload);
    assert_eq!(digest, direct.finalize());
}

#[test]
fn dma_delivered_buffer_can_be_encrypted() {
    let payload = *b"dma cipher block";

    let mut block = deliver_via_dma(payload);
    let mut engine = MockAesEngine::new();
    engine.encrypt_block(&mut block);

    assert_ne!(block, payload, "encryption must change the block");

    // Encrypting the same input directly must produce identical ciphertext.
    let mut direct_block = payload;
    let mut direct_engine = MockAesEngine::new();
    direct_engine.encrypt_block(&mut direct_block);
    assert_eq!(block, direct_block);
}
