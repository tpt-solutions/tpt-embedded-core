//! Safe DMA handle API consumed by `tpt-e-chronos` and `tpt-e-cipher`.
//!
//! This module provides a typestate-based DMA channel abstraction that ensures
//! compile-time safety for DMA operations. The DMA channel transitions through
//! the following states:
//!
//! 1. **Idle**: The channel is powered on but not configured.
//! 2. **Configured**: The channel has been configured with a buffer and transfer parameters.
//! 3. **Transferring**: A DMA transfer is actively in progress.
//! 4. **Complete**: The DMA transfer has completed.
//!
//! Each state transition is enforced at compile time, preventing invalid operations
//! such as starting a transfer without configuration or accessing a buffer during transfer.

use crate::backend::{IdleOps, ConfiguredOps, TransferringOps};
use crate::state::{Complete, Configured, Idle, Transferring};
#[cfg(feature = "mock")]
use crate::mock::MockDmaChannel;

/// A DMA channel parameterised by its current typestate and backend.
///
/// Transitions consume the handle and return a new handle in the next state:
///
/// - `DmaChannel<Idle>`
///   → `configure(...)` → `DmaChannel<Configured>`
///   → `start(...)`    → `DmaChannel<Transferring>`
///   → `wait(...)`     → `DmaChannel<Complete>`
///
/// The `B` type parameter represents the backend implementation. When the
/// `mock` feature is enabled, it defaults to `MockDmaChannel` for host-side
/// testing; otherwise a backend must be specified explicitly.
#[cfg(feature = "mock")]
#[derive(Debug)]
pub struct DmaChannel<State, B = MockDmaChannel> {
    _state: core::marker::PhantomData<State>,
    backend: B,
}

/// A DMA channel parameterised by its current typestate and backend.
///
/// See the `mock`-feature-enabled definition of this type for details;
/// without `mock`, no default backend is provided.
#[cfg(not(feature = "mock"))]
#[derive(Debug)]
pub struct DmaChannel<State, B> {
    _state: core::marker::PhantomData<State>,
    backend: B,
}

impl<B: IdleOps<Configured = B>> DmaChannel<Idle, B> {
    /// Create a new idle DMA channel with the given backend.
    pub fn new(backend: B) -> Self {
        Self { _state: core::marker::PhantomData, backend }
    }

    /// Configure the DMA channel with a buffer and transfer parameters.
    pub fn configure(self, buf: &'static mut [u8], len: usize) -> DmaChannel<Configured, B> {
        DmaChannel {
            _state: core::marker::PhantomData,
            backend: self.backend.configure(buf, len),
        }
    }
}

impl<B: ConfiguredOps<Transferring = B>> DmaChannel<Configured, B> {
    /// Start the DMA transfer.
    pub fn start(self) -> DmaChannel<Transferring, B> {
        DmaChannel {
            _state: core::marker::PhantomData,
            backend: self.backend.start(),
        }
    }
}

impl<B: TransferringOps<Complete = B>> DmaChannel<Transferring, B> {
    /// Wait for the DMA transfer to complete, returning access to the buffer.
    pub fn wait(self) -> DmaChannel<Complete, B> {
        DmaChannel {
            _state: core::marker::PhantomData,
            backend: self.backend.wait(),
        }
    }
}

// Convenience constructors for common backends
#[cfg(feature = "mock")]
impl DmaChannel<Idle, MockDmaChannel> {
    /// Create a new mock DMA channel for host-side testing.
    pub fn mock(channel_id: u8) -> Self {
        Self::new(MockDmaChannel::new(channel_id))
    }
}

#[cfg(feature = "use_esp_hal")]
impl DmaChannel<Idle, crate::backend::EspHalBackend> {
    /// Create a new DMA channel using the esp-hal backend.
    pub fn esp_hal(channel_id: u8) -> Self {
        Self::new(crate::backend::EspHalBackend::new(channel_id))
    }
}