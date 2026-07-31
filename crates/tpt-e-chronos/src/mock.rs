//! Mock implementations for host-side testing.
//!
//! Provides a manual clock source and a convenience wrapper for testing
//! the ring buffer and DMA handoff paths without real hardware.

use core::cell::Cell;

/// A manually-advancing clock for deterministic time-series testing.
///
/// The clock starts at zero and can be advanced in increments. Each call
/// to `now()` returns the current tick count, which is useful for
/// stamping ring buffer entries with reproducible timestamps.
///
/// # Example
///
/// ```rust
/// use tpt_e_chronos::mock::MockClock;
///
/// let clock = MockClock::new();
/// assert_eq!(clock.now(), 0);
/// clock.advance(10);
/// assert_eq!(clock.now(), 10);
/// clock.advance(5);
/// assert_eq!(clock.now(), 15);
/// ```
///
/// Uses [`Cell`] rather than an atomic: this is a single-threaded test
/// utility (no ISR/main-loop sharing claim, unlike [`crate::ring_buf::RingBuf`]),
/// and `AtomicU64` doesn't exist on some real target chips this crate
/// compiles for — `riscv32imc-unknown-none-elf` (ESP32-C3) lacks the
/// atomic-RMW extension and only exposes `load`/`store` on `core::sync::atomic`
/// types, not `fetch_add`. That gap went uncaught here because CI never
/// cross-compiled the `mock` feature to a real chip target (see the root
/// `todo.md`'s note that this exact bug class — `fetch_add`/`compare_exchange`
/// under `mock`-or-similar on `riscv32imc` — was previously found and fixed
/// in `tpt-e-slumber`'s mock and flagged there as unaudited elsewhere).
/// `Cell` has no atomicity requirement at all, so it sidesteps the problem
/// on every target rather than working around it per-target.
#[derive(Debug)]
pub struct MockClock {
    ticks: Cell<u64>,
}

impl MockClock {
    /// Create a new mock clock starting at tick 0.
    pub fn new() -> Self {
        Self { ticks: Cell::new(0) }
    }

    /// Advance the clock by the given number of ticks.
    pub fn advance(&self, ticks: u64) {
        self.ticks.set(self.ticks.get() + ticks);
    }

    /// Get the current tick count.
    pub fn now(&self) -> u64 {
        self.ticks.get()
    }

    /// Reset the clock to zero.
    pub fn reset(&self) {
        self.ticks.set(0);
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

/// A timestamped entry for use with [`MockClock`].
///
/// Pairs a monotonically increasing timestamp with a data payload,
/// useful for testing ordered time-series scenarios in the ring buffer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Timestamped<T> {
    /// The tick at which this entry was recorded.
    pub timestamp: u64,
    /// The data payload.
    pub value: T,
}

impl<T> Timestamped<T> {
    /// Create a new timestamped entry.
    pub fn new(timestamp: u64, value: T) -> Self {
        Self { timestamp, value }
    }
}

/// Convenience helper: push a timestamped value into a ring buffer.
///
/// Returns `Ok(())` on success, or `Err(item)` if the buffer is full.
pub fn push_ts<T: Copy, const CAP: usize>(
    buf: &crate::ring_buf::RingBuf<Timestamped<T>, CAP>,
    clock: &MockClock,
    value: T,
) -> Result<(), Timestamped<T>> {
    buf.push(Timestamped::new(clock.now(), value))
}
