//! Safe ISR abstractions.

#[cfg(feature = "mock")]
use crate::mock::MockIsrGuard;

/// An interrupt service routine registration guard.
///
/// While this handle exists, the ISR is registered and active.
/// Dropping the handle unregisters the ISR.
///
/// The `B` type parameter represents the backend implementation. When the
/// `mock` feature is enabled, it defaults to `MockIsrGuard` for host-side
/// testing; otherwise a backend must be specified explicitly.
#[cfg(feature = "mock")]
#[allow(missing_debug_implementations)]
pub struct IsrGuard<F: Fn(), B = MockIsrGuard<F>> {
    backend: B,
    _handler: core::marker::PhantomData<F>,
}

/// An interrupt service routine registration guard.
///
/// See the `mock`-feature-enabled definition of this type for details;
/// without `mock`, no default backend is provided.
#[cfg(not(feature = "mock"))]
#[allow(missing_debug_implementations)]
pub struct IsrGuard<F: Fn(), B> {
    backend: B,
    _handler: core::marker::PhantomData<F>,
}

#[cfg(feature = "mock")]
impl<F: Fn()> IsrGuard<F, MockIsrGuard<F>> {
    /// Register a mock ISR for host-side testing.
    pub fn mock(handler: F) -> Self {
        Self {
            backend: MockIsrGuard::new(handler),
            _handler: core::marker::PhantomData,
        }
    }

    /// Call the mock handler (for testing purposes).
    pub fn call_mock(&self) {
        self.backend.call();
    }
}

#[cfg(feature = "use_esp_hal")]
impl<F: Fn()> IsrGuard<F, crate::backend::EspHalIsrGuard<F>> {
    /// Register an ISR using the esp-hal backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure the closure is suitable for ISR context
    /// (no blocking, no allocation, bounded execution time).
    pub unsafe fn esp_hal(handler: F) -> Self {
        Self {
            backend: crate::backend::EspHalIsrGuard::new(handler),
            _handler: core::marker::PhantomData,
        }
    }
}

// Generic implementation for any backend
impl<F: Fn(), B> IsrGuard<F, B> {
    /// Create a new ISR guard with the given backend.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            _handler: core::marker::PhantomData,
        }
    }
}

impl<F: Fn(), B> Drop for IsrGuard<F, B> {
    fn drop(&mut self) {
        // The backend's Drop implementation handles cleanup
    }
}