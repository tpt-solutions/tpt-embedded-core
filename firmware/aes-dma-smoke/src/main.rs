//! Real-hardware DMA smoke test for `tpt-embedded-core`.
//!
//! Unlike `hil-hello` (which just proves the flash/monitor pipeline works),
//! this drives an actual peripheral — the ESP32-C3's hardware AES engine —
//! through DMA (`esp_hal::aes::dma::AesDma`), then compares the result
//! against the FIPS-197 Appendix B known-answer vector (the same constant
//! already used and verified in `tpt-e-cipher/src/aes.rs::aes128_nist_vector`
//! against the crate's software AES implementation). A match proves a real
//! DMA transfer moved real data into and out of a real hardware peripheral
//! correctly on this board.
//!
//! No external wiring needed: AES-DMA is entirely internal to the chip
//! (RAM buffers <-> DMA <-> AES engine), unlike e.g. SPI-DMA loopback,
//! which would need a physical MOSI-MISO jumper.
//!
//! This is a data point toward (not a replacement for) the still-open
//! `tpt-e-typestate-hal` `EspHalBackend` real-DMA-implementation gap in the
//! root `todo.md`: it confirms the underlying esp-hal DMA+AES API this repo
//! would eventually wrap actually works on real silicon.

#![no_std]
#![no_main]

use esp_hal::{
    aes::{dma::CipherMode, Aes, Mode},
    delay::Delay,
    dma::{Dma, DmaPriority},
    dma_descriptors,
    prelude::*,
    usb_serial_jtag::UsbSerialJtag,
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// FIPS-197 Appendix B known-answer vector — the exact same constants
// verified in `tpt-e-cipher/src/aes.rs::aes128_nist_vector` against this
// repo's own software AES-128 implementation.
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

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0.configure(false, DmaPriority::Priority0);
    let (rx_descriptors, tx_descriptors) = dma_descriptors!(16, 16);

    let aes = Aes::new(peripherals.AES);
    let mut aes_dma = aes.with_dma(dma_channel, rx_descriptors, tx_descriptors);

    let mut ciphertext = [0u8; 16];
    let result = aes_dma
        .process(
            &PLAINTEXT,
            &mut ciphertext,
            Mode::Encryption128,
            CipherMode::Ecb,
            KEY,
        )
        .and_then(|transfer| transfer.wait());

    let dma_ok = result.is_ok();
    let matches = ciphertext == EXPECTED_CIPHERTEXT;

    loop {
        if !dma_ok {
            let _ = usb_serial.write_bytes(b"AES-DMA FAIL: DMA transfer returned an error\r\n");
        } else if matches {
            let _ = usb_serial
                .write_bytes(b"AES-DMA PASS: hardware ciphertext matches FIPS-197 vector\r\n");
        } else {
            let _ = usb_serial
                .write_bytes(b"AES-DMA FAIL: hardware ciphertext does NOT match expected\r\n");
            let _ = usb_serial.write_bytes(b"  got: ");
            write_hex(&mut usb_serial, &ciphertext);
            let _ = usb_serial.write_bytes(b"\r\n  exp: ");
            write_hex(&mut usb_serial, &EXPECTED_CIPHERTEXT);
            let _ = usb_serial.write_bytes(b"\r\n");
        }
        delay.delay_millis(1000);
    }
}

/// Writes `bytes` as lowercase hex over the USB Serial/JTAG console.
fn write_hex(usb_serial: &mut UsbSerialJtag<'_, esp_hal::Blocking>, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &b in bytes {
        let pair = [HEX[(b >> 4) as usize], HEX[(b & 0xf) as usize]];
        let _ = usb_serial.write_bytes(&pair);
    }
}
