//! Sleep state machine gated on proof tokens.
//!
//! The `SleepController` manages power transitions for the ESP32 family.
//! Deep sleep is only accessible when all required proof tokens are provided
//! as parameters — missing tokens produce a compile error.

use crate::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

/// The sleep controller.
///
/// Created via `SleepController::new()`. Deep sleep transitions are
/// gated on proof tokens that must be obtained from their respective
/// subsystems.
#[derive(Debug, Copy, Clone)]
pub struct SleepController;

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
    /// This function returns `!` (never type) — the device enters deep sleep
    /// and does not return to the caller.
    pub fn enter_deep_sleep(
        self,
        _dma: DmaParkedToken,
        _rtc: RtcIsolatedToken,
        _buffers: BuffersFlushedToken,
    ) -> ! {
        // In a real implementation, this would:
        // 1. Flush any remaining caches
        // 2. Isolate RTC memory
        // 3. Park all DMA channels
        // 4. Execute the ESP32 deep sleep instruction
        //
        // Since this is no_std and the hardware instruction is platform-specific,
        // we use an infinite loop as the placeholder.
        loop {}
    }
}

impl Default for SleepController {
    fn default() -> Self {
        Self::new()
    }
}
