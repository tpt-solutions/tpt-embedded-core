//! Real-hardware smoke test for `tpt_e_typestate_hal::aes_dma::AesDmaChannel`.
//!
//! Unlike `aes-dma-smoke` (which drives `esp_hal::aes::dma::AesDma`
//! directly, proving only that the underlying esp-hal mechanism works),
//! this drives the same hardware through `tpt-e-typestate-hal`'s own
//! `AesDmaChannel` typestate wrapper (`Idle -> Configured -> Transferring ->
//! Complete`) — the actual crate API this repo ships, added 2026-07-30 to
//! close the long-standing `EspHalBackend` real-DMA-implementation gap in
//! the root `todo.md`. A match against the same FIPS-197 Appendix B
//! known-answer vector proves the *wrapper*, not just the raw esp-hal call,
//! is correct on real silicon.

#![no_std]
#![no_main]

use esp_hal::{delay::Delay, dma::DmaPriority, dma_descriptors, prelude::*, usb_serial_jtag::UsbSerialJtag};
use tpt_e_typestate_hal::aes_dma::AesDmaChannel;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// FIPS-197 Appendix B known-answer vector -- the same constants used in
// `aes-dma-smoke` and verified in `tpt-e-cipher/src/aes.rs::aes128_nist_vector`.
const KEY: [u8; 16] = [
    0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6, 0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C,
];
const PLAINTEXT: [u8; 16] = [
    0x32, 0x43, 0xF6, 0xA8, 0x88, 0x5A, 0x30, 0x8D, 0x31, 0x31, 0x98, 0xA2, 0xE0, 0x37, 0x07, 0x34,
];
const EXPECTED_CIPHERTEXT: [u8; 16] = [
    0x39, 0x25, 0x84, 0x1D, 0x02, 0xDC, 0x09, 0xFB, 0xDC, 0x11, 0x85, 0x97, 0x19, 0x6A, 0x0B, 0x32,
];

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE);
    let delay = Delay::new();

    let dma = esp_hal::dma::Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0.configure(false, DmaPriority::Priority0);
    let (rx_descriptors, tx_descriptors) = dma_descriptors!(16, 16);

    let aes = esp_hal::aes::Aes::new(peripherals.AES);
    let aes_dma = aes.with_dma(dma_channel, rx_descriptors, tx_descriptors);

    // Drive the real typestate chain: Idle -> Configured -> Transferring -> Complete.
    let channel = AesDmaChannel::new(aes_dma);
    let channel = channel.configure(esp_hal::aes::Mode::Encryption128, esp_hal::aes::dma::CipherMode::Ecb);

    let mut ciphertext = [0u8; 16];
    let channel = channel.start(KEY, &PLAINTEXT, &mut ciphertext);
    let channel = channel.wait();

    let dma_ok = channel.result().is_ok();
    let matches = ciphertext == EXPECTED_CIPHERTEXT;

    loop {
        if !dma_ok {
            let _ = usb_serial
                .write_bytes(b"typestate AES-DMA FAIL: DMA transfer returned an error\r\n");
        } else if matches {
            let _ = usb_serial.write_bytes(
                b"typestate AES-DMA PASS: AesDmaChannel ciphertext matches FIPS-197 vector\r\n",
            );
        } else {
            let _ = usb_serial
                .write_bytes(b"typestate AES-DMA FAIL: ciphertext does NOT match expected\r\n");
        }
        delay.delay_millis(1000);
    }
}
