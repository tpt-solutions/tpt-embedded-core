//! Real-hardware smoke test for plain ESP32 (Xtensa).
//!
//! Like `../hil-hello`, this exists solely to prove the
//! toolchain/build/flash/monitor pipeline works end to end against real
//! ESP32 silicon. Unlike the RISC-V firmware crates in this workspace
//! (`aes-dma-smoke`, `slumber-smoke`, `typestate-aes-dma-smoke`), which
//! exercise actual peripheral DMA and library-crate APIs, this is a
//! minimal "alive" counter — the original ESP32 has no built-in USB
//! Serial/JTAG, so it prints over UART0 (GPIO1 TX / GPIO3 RX) instead.
//!
//! Run with: `cargo run --release` from this directory (see
//! `.cargo/config.toml` for the `espflash flash --monitor` runner).

#![no_std]
#![no_main]

use esp_hal::{delay::Delay, main};

#[cfg(feature = "esp32")]
use esp_hal::uart::Uart as Console;
#[cfg(not(feature = "esp32"))]
use esp_hal::usb_serial_jtag::UsbSerialJtag as Console;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    #[cfg(feature = "esp32")]
    let mut console = Console::new(peripherals.UART0, esp_hal::uart::Config::default())
        .expect("UART0 init failed");
    #[cfg(not(feature = "esp32"))]
    let mut console = Console::new(peripherals.USB_DEVICE);

    let delay = Delay::new();

    let mut count: u32 = 0;
    loop {
        let _ = console.write_bytes(b"esp32-smoke: alive (count=");
        let mut buf = [0u8; 10];
        let s = write_u32(&mut buf, count);
        let _ = console.write_bytes(s);
        let _ = console.write_bytes(b")\r\n");

        count = count.wrapping_add(1);
        delay.delay_millis(1000);
    }
}

/// Formats `n` as decimal ASCII into `buf`, returning the written slice.
fn write_u32(buf: &mut [u8; 10], mut n: u32) -> &[u8] {
    if n == 0 {
        buf[0] = b'0';
        return &buf[..1];
    }
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}
