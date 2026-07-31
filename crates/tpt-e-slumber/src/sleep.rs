//! Sleep state machine gated on proof tokens.
//!
//! The `SleepController` manages power transitions for the ESP32 family.
//! Deep sleep is only accessible when all required proof tokens are provided
//! as parameters — missing tokens produce a compile error.

#[cfg(feature = "defmt")]
use defmt::trace;

use crate::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

/// The sleep controller.
///
/// Created via `SleepController::new()`. Deep sleep transitions are
/// gated on proof tokens that must be obtained from their respective
/// subsystems.
///
/// Without the `use_esp_hal` feature, `enter_deep_sleep` has nothing to
/// call on host builds and instead parks in an infinite loop after
/// consuming the tokens — that path exists only so `mock`/host tests can
/// exercise the token-gating without linking real hardware peripherals.
/// With `use_esp_hal` enabled, this wraps a real `esp_hal::rtc_cntl::Rtc`
/// and `enter_deep_sleep` executes the actual deep-sleep instruction via
/// `Rtc::sleep_deep`.
#[cfg(not(feature = "use_esp_hal"))]
#[derive(Debug, Copy, Clone)]
pub struct SleepController;

#[cfg(not(feature = "use_esp_hal"))]
impl SleepController {
    /// Create a new sleep controller.
    pub fn new() -> Self {
        Self
    }

    /// Enter deep sleep.
    ///
    /// This function can only be called when all required proof tokens
    /// have been obtained — missing tokens produce a compile-time error.
    ///
    /// # Required Tokens
    ///
    /// - `DmaParkedToken`: Proves all DMA channels are parked.
    /// - `RtcIsolatedToken`: Proves RTC memory is isolated.
    /// - `BuffersFlushedToken`: Proves all transmit buffers are flushed.
    ///
    /// # Never Returns
    ///
    /// This function returns `!` (never type). On real hardware
    /// (`use_esp_hal` feature) the device enters deep sleep and does not
    /// return. This host-only build has no hardware sleep instruction to
    /// execute, so it parks in an infinite loop instead.
    pub fn enter_deep_sleep(
        self,
        _dma: DmaParkedToken,
        _rtc: RtcIsolatedToken,
        _buffers: BuffersFlushedToken,
    ) -> ! {
        loop {}
    }
}

#[cfg(not(feature = "use_esp_hal"))]
impl Default for SleepController {
    fn default() -> Self {
        Self::new()
    }
}

/// The sleep controller (real hardware backend).
///
/// Wraps an `esp_hal::rtc_cntl::Rtc` obtained by the caller (typically via
/// `Rtc::new(peripherals.LPWR)` after `esp_hal::init`). Constructing this
/// requires an actual peripheral handle, so unlike the host-only build
/// there is no zero-argument `new()`.
#[cfg(feature = "use_esp_hal")]
#[allow(missing_copy_implementations)] // wraps a non-Copy hardware peripheral handle
pub struct SleepController {
    rtc: esp_hal::rtc_cntl::Rtc<'static>,
}

// `esp_hal::rtc_cntl::Rtc` doesn't implement `Debug`, so this is written by
// hand instead of derived.
#[cfg(feature = "use_esp_hal")]
impl core::fmt::Debug for SleepController {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SleepController").finish_non_exhaustive()
    }
}

#[cfg(feature = "use_esp_hal")]
impl SleepController {
    /// Create a new sleep controller wrapping the given RTC peripheral handle.
    pub fn new(rtc: esp_hal::rtc_cntl::Rtc<'static>) -> Self {
        #[cfg(feature = "defmt")]
        trace!("SleepController::new: wrapping RTC handle");
        Self { rtc }
    }

    /// Enter deep sleep.
    ///
    /// This function can only be called when all required proof tokens
    /// have been obtained — missing tokens produce a compile-time error.
    ///
    /// # Required Tokens
    ///
    /// - `DmaParkedToken`: Proves all DMA channels are parked.
    /// - `RtcIsolatedToken`: Proves RTC memory is isolated.
    /// - `BuffersFlushedToken`: Proves all transmit buffers are flushed.
    ///
    /// # Never Returns
    ///
    /// This function returns `!` (never type) — it executes the real RTC
    /// deep-sleep instruction (`Rtc::sleep_deep`) with no configured wake
    /// sources, so the device enters deep sleep and does not return.
    pub fn enter_deep_sleep(
        mut self,
        _dma: DmaParkedToken,
        _rtc: RtcIsolatedToken,
        _buffers: BuffersFlushedToken,
    ) -> ! {
        #[cfg(feature = "defmt")]
        trace!("SleepController::enter_deep_sleep: executing RTC deep-sleep instruction");
        self.rtc.sleep_deep(&[]);
    }
}
