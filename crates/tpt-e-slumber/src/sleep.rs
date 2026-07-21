//! Sleep state machine gated on proof tokens.

use crate::tokens::{DmaParkedToken, RtcIsolatedToken};

/// The sleep controller.
#[derive(Debug, Copy, Clone)]
pub struct SleepController;

impl SleepController {
    /// Enter deep sleep.
    ///
    /// This function can only be called when all required proof tokens
    /// have been obtained — missing tokens produce a compile-time error.
    pub fn enter_deep_sleep(
        self,
        _dma: DmaParkedToken,
        _rtc: RtcIsolatedToken,
    ) -> ! {
        loop {}
    }
}
