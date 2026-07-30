//! Real-hardware smoke test for `tpt-e-slumber`.
//!
//! Unlike `hil-hello` (pipeline-only) and `aes-dma-smoke` (drives esp-hal
//! directly, not through any `tpt-e-*` crate API), this calls into
//! `tpt_e_slumber::sleep::SleepController::enter_deep_sleep` — the crate's
//! actual public API — with `use_esp_hal` enabled, so it exercises the real
//! `Rtc::sleep_deep` hardware path added 2026-07-29 (see the root
//! `todo.md`), not just a compile check.
//!
//! Proof tokens are obtained via `Token::mock()` (the `mock` feature), not
//! from a real precondition-checked issuer — that integration
//! (`tpt-e-typestate-hal`/RTC/UART driver wiring) is still a separate open
//! item. This test is scoped narrowly: does the real RTC deep-sleep
//! instruction, once reached, actually put the chip to sleep? It does not
//! test token issuance.
//!
//! `enter_deep_sleep` calls `Rtc::sleep_deep(&[])` — no wake sources — so
//! once asleep the chip stays asleep until an external reset (EN button or
//! power cycle). Expected observable behavior: the countdown prints, then
//! serial output stops permanently (not just pauses) until the board is
//! manually reset.

#![no_std]
#![no_main]

use esp_hal::{delay::Delay, prelude::*, rtc_cntl::Rtc, usb_serial_jtag::UsbSerialJtag};
use tpt_e_slumber::{
    sleep::SleepController,
    tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken},
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE);
    let delay = Delay::new();

    let _ = usb_serial.write_bytes(b"tpt-e-slumber HIL smoke test\r\n");
    let _ = usb_serial.write_bytes(
        b"Will call SleepController::enter_deep_sleep in real hardware in 3 seconds.\r\n",
    );
    let _ = usb_serial.write_bytes(
        b"No wake source is configured, so the board will NOT wake on its own -- \r\n",
    );
    let _ = usb_serial.write_bytes(b"press RESET/EN to reboot afterward.\r\n");

    for n in (1..=3).rev() {
        let _ = usb_serial.write_bytes(b"  entering deep sleep in ");
        let _ = usb_serial.write_bytes(&[b'0' + n]);
        let _ = usb_serial.write_bytes(b"...\r\n");
        delay.delay_millis(1000);
    }

    let rtc = Rtc::new(peripherals.LPWR);
    let controller = SleepController::new(rtc);

    let dma_token = DmaParkedToken::mock();
    let rtc_token = RtcIsolatedToken::mock();
    let buffers_token = BuffersFlushedToken::mock();

    let _ = usb_serial.write_bytes(b"Calling enter_deep_sleep() now. If you see this line\r\n");
    let _ = usb_serial.write_bytes(b"but nothing after it, the hardware sleep instruction\r\n");
    let _ = usb_serial.write_bytes(b"executed successfully (this function returns `!`).\r\n");

    controller.enter_deep_sleep(dma_token, rtc_token, buffers_token);
}
