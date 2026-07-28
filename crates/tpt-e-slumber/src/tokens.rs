//! Safety proof tokens for sleep transitions.
//!
//! Tokens are zero-sized types that can only be created by subsystems that
//! have proven their safety precondition is satisfied. This ensures that
//! deep sleep is only entered when all peripherals are safely parked.
//!
//! # Typestate Guarantee
//!
//! The `SleepController::enter_deep_sleep` method requires all tokens as
//! parameters. If any token is missing, the code fails to compile — not at
//! runtime. This is the core compile-time safety guarantee of `tpt-e-slumber`.
//!
//! # Linear type
//!
//! Tokens do **not** implement `Copy` or `Clone`. Once consumed (e.g. by
//! `enter_deep_sleep` or `try_sleep`), a token cannot be reused. Attempting
//! to derive `Copy` on a token will fail because the trait is not implemented.

/// Token proving DMA has been safely parked.
///
/// Issued by `tpt-e-typestate-hal` when all DMA channels are in the `Complete`
/// or `Idle` state and no transfers are in progress.
///
/// # Linear type
///
/// This token does **not** implement `Copy` or `Clone`. Once consumed (e.g. by
/// `enter_deep_sleep` or `try_sleep`), it cannot be reused. This prevents a
/// token obtained when a precondition held from being duplicated and later used
/// after the precondition was invalidated (e.g. DMA restarted after a
/// `DmaParkedToken` was obtained).
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct DmaParkedToken {
    _private: (),
}

impl DmaParkedToken {
    /// Create a new DMA parked token.
    ///
    /// # Safety Contract
    ///
    /// The caller (typically `tpt-e-typestate-hal`) must guarantee that:
    /// - All DMA channels have completed their transfers.
    /// - No new DMA transfers will be started after this token is issued.
    /// - All DMA buffers have been flushed to memory.
    ///
    /// Restricted to `pub(crate)`: no subsystem in this workspace issues
    /// this token from a verified precondition yet (the DMA/RTC/UART driver
    /// integration is still TODO), so it is deliberately not constructible
    /// from outside the crate. Use [`DmaParkedToken::mock`] under the
    /// `mock` feature for host-side testing.
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    /// Create a DMA parked token for host-side testing.
    ///
    /// Only available under the `mock` feature — does not prove any real
    /// precondition, unlike a token issued by a real driver.
    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self::new()
    }
}

/// Token proving RTC memory has been isolated.
///
/// Issued when RTC memory regions are properly isolated and no pending
/// writes remain that could corrupt RTC state during sleep.
///
/// # Linear type
///
/// This token does **not** implement `Copy` or `Clone`. See
/// [`DmaParkedToken`] for rationale.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct RtcIsolatedToken {
    _private: (),
}

impl RtcIsolatedToken {
    /// Create a new RTC isolated token.
    ///
    /// # Safety Contract
    ///
    /// The caller must guarantee that:
    /// - All RTC memory writes have completed.
    /// - RTC memory regions are properly isolated for sleep.
    /// - No DMA or other peripheral is writing to RTC memory.
    ///
    /// Restricted to `pub(crate)` — see [`DmaParkedToken::new`] for why. Use
    /// [`RtcIsolatedToken::mock`] under the `mock` feature for testing.
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    /// Create an RTC isolated token for host-side testing.
    ///
    /// Only available under the `mock` feature — does not prove any real
    /// precondition, unlike a token issued by a real driver.
    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self::new()
    }
}

/// Token proving all UART/TX buffers have been flushed.
///
/// Issued by the UART driver when all transmit buffers are empty and
/// no further data will be written.
///
/// # Linear type
///
/// This token does **not** implement `Copy` or `Clone`. See
/// [`DmaParkedToken`] for rationale.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct BuffersFlushedToken {
    _private: (),
}

impl BuffersFlushedToken {
    /// Create a new buffers flushed token.
    ///
    /// # Safety Contract
    ///
    /// The caller must guarantee that:
    /// - All UART transmit buffers are empty.
    /// - No pending DMA transfers to UART peripherals.
    ///
    /// Restricted to `pub(crate)` — see [`DmaParkedToken::new`] for why. Use
    /// [`BuffersFlushedToken::mock`] under the `mock` feature for testing.
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    /// Create a buffers-flushed token for host-side testing.
    ///
    /// Only available under the `mock` feature — does not prove any real
    /// precondition, unlike a token issued by a real driver.
    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self::new()
    }
}
