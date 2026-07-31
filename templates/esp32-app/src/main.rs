//! Minimal application scaffold built on `tpt-embedded-core`.
//!
//! Demonstrates the core patterns:
//! - `tpt_e_typestate_hal`'s typestate DMA channel (`Idle → Configured →
//!   Transferring → Complete`)
//! - `tpt_e_chronos::RingBuf` ISR-safe push / main-loop pop
//! - `tpt_e_cipher::MockSha256Engine` hashing over ring-buffer data
//!
//! This template uses the `mock` feature so it runs on host without hardware.
//! Swap to real esp-hal backends by enabling the appropriate `use_esp_hal`
//! and chip features.

#![no_std]
#![no_main]

use esp_hal::prelude::*;
use tpt_e_chronos::ring_buf::RingBuf;
use tpt_e_cipher::traits::Sha256;
use tpt_e_cipher::mock::MockSha256Engine;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default());

    // -----------------------------------------------------------------------
    // 1. Ring buffer — ISR-safe push / main-loop pop
    // -----------------------------------------------------------------------
    let telemetry: RingBuf<u32, 64> = RingBuf::new(0);

    // Simulate ISR push of telemetry samples.
    for i in 0..32u32 {
        let _ = telemetry.push(i);
    }

    // Main-loop drain: hash all samples together via SHA-256.
    let mut hasher = MockSha256Engine::new();
    while let Some(sample) = telemetry.pop() {
        hasher.update(&sample.to_le_bytes());
    }
    let _digest = hasher.finalize();

    // -----------------------------------------------------------------------
    // 2. (Placeholder) Typestate DMA channel
    // -----------------------------------------------------------------------
    // On real hardware with `use_esp_hal` enabled, construct a real
    // `AesDmaChannel` and drive it through the full typestate chain:
    //
    //   let channel = AesDmaChannel::new(aes_dma);
    //   let channel = channel.configure(Mode::Encryption128, CipherMode::Ecb);
    //   let channel = channel.start(key, plaintext, &mut ciphertext);
    //   let channel = channel.wait();
    //   let result = channel.result();

    loop {
        // On real hardware, enter deep sleep here with proof tokens from
        // `tpt-e-slumber`:
        //
        //   controller.enter_deep_sleep(dma_token, rtc_token, buffers_token);
    }
}
