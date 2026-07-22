//! Mock implementations for host-side testing of sleep transitions.
//!
//! Provides a recordable sleep backend that tracks whether `enter_deep_sleep`
//! was invoked and which tokens were provided, without actually entering
//! hardware sleep.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

/// A mock sleep backend that records sleep attempts.
///
/// Since the real `SleepController::enter_deep_sleep` returns `!` (never),
/// this mock provides a separate `try_sleep` path that can be used in
/// tests to verify token-gating logic without diverging.
///
/// # Example
///
/// ```rust
/// use tpt_e_slumber::mock::MockSleepBackend;
/// use tpt_e_slumber::tokens::{DmaParkedToken, RtcIsolatedToken, BuffersFlushedToken};
///
/// let backend = MockSleepBackend::new();
/// assert!(!backend.was_sleep_attempted());
///
/// let dma = DmaParkedToken::mock();
/// let rtc = RtcIsolatedToken::mock();
/// let buf = BuffersFlushedToken::mock();
/// backend.try_sleep(dma, rtc, buf);
///
/// assert!(backend.was_sleep_attempted());
/// assert_eq!(backend.sleep_count(), 1);
/// ```
#[derive(Debug)]
pub struct MockSleepBackend {
    attempted: AtomicBool,
    count: AtomicUsize,
}

impl MockSleepBackend {
    /// Create a new mock sleep backend.
    pub fn new() -> Self {
        Self {
            attempted: AtomicBool::new(false),
            count: AtomicUsize::new(0),
        }
    }

    /// Attempt a mock deep sleep.
    ///
    /// Consumes the proof tokens (same as the real `enter_deep_sleep`)
    /// but does not diverge — returns normally so tests can inspect
    /// the result.
    pub fn try_sleep(
        &self,
        _dma: DmaParkedToken,
        _rtc: RtcIsolatedToken,
        _buffers: BuffersFlushedToken,
    ) {
        self.attempted.store(true, Ordering::Relaxed);
        let _ = self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns `true` if `try_sleep` has been called at least once.
    pub fn was_sleep_attempted(&self) -> bool {
        self.attempted.load(Ordering::Relaxed)
    }

    /// Returns the number of times `try_sleep` has been called.
    pub fn sleep_count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Reset the backend to its initial state.
    pub fn reset(&self) {
        self.attempted.store(false, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);
    }
}

impl Default for MockSleepBackend {
    fn default() -> Self {
        Self::new()
    }
}
