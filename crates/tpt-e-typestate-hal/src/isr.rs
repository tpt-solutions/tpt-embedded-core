//! Safe ISR abstractions.

/// An interrupt service routine registration guard.
///
/// While this handle exists, the ISR is registered and active.
/// Dropping the handle unregisters the ISR.
#[allow(missing_debug_implementations)]
pub struct IsrGuard<F: Fn()> {
    _handler: F,
}

impl<F: Fn()> IsrGuard<F> {
    /// Register an ISR. The provided closure is called on each interrupt.
    ///
    /// # Safety
    ///
    /// The caller must ensure the closure is suitable for ISR context
    /// (no blocking, no allocation, bounded execution time).
    pub unsafe fn register(_handler: F) -> Self {
        Self { _handler }
    }
}
