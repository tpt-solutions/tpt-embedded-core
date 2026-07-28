//! Chip-agnostic abstraction layer over DMA and ISR primitives.
//!
//! This module defines traits that abstract over the low-level hardware operations,
//! allowing the same typestate API to work across different ESP32 chips and with
//! mock implementations for host-side testing.
//!
//! # Safety
//!
//! The traits defined in this module are safe to implement. However, the actual
//! hardware implementations (esp-hal backend) contain unavoidable `unsafe` code
//! for register access. These unsafe blocks are isolated in the backend implementations
//! and must adhere to the documented safety invariants.

#![allow(unsafe_code)]

/// Trait for DMA channel operations in the Idle state.
pub trait IdleOps {
    /// The type returned after configuration.
    type Configured;

    /// Configure the DMA channel with a buffer and transfer parameters.
    fn configure(self, buf: &'static mut [u8], len: usize) -> Self::Configured;
}

/// Trait for DMA channel operations in the Configured state.
pub trait ConfiguredOps {
    /// The type returned after starting the transfer.
    type Transferring;

    /// Start the DMA transfer.
    fn start(self) -> Self::Transferring;
}

/// Trait for DMA channel operations in the Transferring state.
pub trait TransferringOps {
    /// The type returned after waiting for completion.
    type Complete;

    /// Wait for the DMA transfer to complete, returning access to the buffer.
    fn wait(self) -> Self::Complete;
}

/// Trait for ISR registration operations.
///
/// Generic over the handler type `F` at the *trait* level (not the method
/// level): an implementor is a concrete guard type for one particular `F`
/// (e.g. `MockIsrGuard<F>`, `EspHalIsrGuard<F>`), which is the shape ISR
/// guards in this crate actually take. A method-level `fn register<F: Fn()>(..)
/// -> Self` (the crate's original design) can't be implemented by either
/// guard, since `Self` would need to vary with `F` — this is why, until this
/// fix, the trait existed but had zero implementors and the guards used
/// unrelated inherent constructors (`MockIsrGuard::new`,
/// `EspHalIsrGuard::new`) instead.
pub trait IsrOps<F: Fn()>: Sized {
    /// Register an ISR handler. The provided closure is called on each interrupt.
    ///
    /// # Safety
    ///
    /// The caller must ensure the closure is suitable for ISR context
    /// (no blocking, no allocation, bounded execution time).
    unsafe fn register(handler: F) -> Self;
}

/// Esp-hal backend for real hardware (conditional on `use_esp_hal` feature).
#[cfg(feature = "use_esp_hal")]
pub struct EspHalBackend {
    channel_id: u8,
}

#[cfg(feature = "use_esp_hal")]
impl EspHalBackend {
    /// Create a new esp-hal backend with the given channel ID.
    pub fn new(channel_id: u8) -> Self {
        Self { channel_id }
    }

    /// Get the channel ID.
    pub fn channel_id(&self) -> u8 {
        self.channel_id
    }
}

// Esp-hal backend trait implementations
#[cfg(feature = "use_esp_hal")]
impl IdleOps for EspHalBackend {
    type Configured = Self;

    fn configure(self, _buf: &'static mut [u8], _len: usize) -> Self {
        // TODO: Implement actual esp-hal DMA configuration
        // This is a placeholder that will be implemented when we have access to
        // the specific esp-hal API for the target chips.
        self
    }
}

#[cfg(feature = "use_esp_hal")]
impl ConfiguredOps for EspHalBackend {
    type Transferring = Self;

    fn start(self) -> Self {
        // TODO: Implement actual esp-hal DMA start
        self
    }
}

#[cfg(feature = "use_esp_hal")]
impl TransferringOps for EspHalBackend {
    type Complete = Self;

    fn wait(self) -> Self {
        // TODO: Implement actual esp-hal DMA wait
        self
    }
}

/// Esp-hal ISR guard for real hardware (conditional on `use_esp_hal` feature).
#[cfg(feature = "use_esp_hal")]
pub struct EspHalIsrGuard<F: Fn()> {
    _handler: F,
}

#[cfg(feature = "use_esp_hal")]
impl<F: Fn()> EspHalIsrGuard<F> {
    /// Create a new esp-hal ISR guard.
    ///
    /// # Safety
    ///
    /// The caller must ensure the closure is suitable for ISR context.
    pub unsafe fn new(handler: F) -> Self {
        // TODO: Implement actual esp-hal ISR registration
        Self { _handler: handler }
    }
}

#[cfg(feature = "use_esp_hal")]
impl<F: Fn()> IsrOps<F> for EspHalIsrGuard<F> {
    unsafe fn register(handler: F) -> Self {
        // SAFETY: forwards the same precondition `register`'s caller must
        // uphold (closure suitable for ISR context) to `Self::new`.
        unsafe { Self::new(handler) }
    }
}

#[cfg(feature = "use_esp_hal")]
impl<F: Fn()> Drop for EspHalIsrGuard<F> {
    fn drop(&mut self) {
        // TODO: Implement actual esp-hal ISR unregistration
    }
}