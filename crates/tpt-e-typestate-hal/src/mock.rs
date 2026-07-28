//! Mock implementations for host-side testing.

use crate::backend::{IdleOps, ConfiguredOps, TransferringOps, IsrOps};

/// A mock DMA channel for host-side testing.
#[derive(Debug, Copy, Clone)]
pub struct MockDmaChannel {
    channel_id: u8,
}

impl MockDmaChannel {
    /// Create a new mock DMA channel with the given ID.
    pub fn new(channel_id: u8) -> Self {
        Self { channel_id }
    }

    /// Get the channel ID.
    pub fn channel_id(&self) -> u8 {
        self.channel_id
    }
}

impl IdleOps for MockDmaChannel {
    type Configured = Self;

    fn configure(self, _buf: &'static mut [u8], _len: usize) -> Self {
        // Mock implementation: just return self
        self
    }
}

impl ConfiguredOps for MockDmaChannel {
    type Transferring = Self;

    fn start(self) -> Self {
        // Mock implementation: just return self
        self
    }
}

impl TransferringOps for MockDmaChannel {
    type Complete = Self;

    fn wait(self) -> Self {
        // Mock implementation: just return self
        self
    }
}

/// Mock ISR guard for host-side testing.
#[derive(Debug)]
pub struct MockIsrGuard<F: Fn()> {
    handler: F,
}

impl<F: Fn()> MockIsrGuard<F> {
    /// Create a new mock ISR guard.
    pub fn new(handler: F) -> Self {
        Self { handler }
    }

    /// Call the handler (for testing purposes).
    pub fn call(&self) {
        (self.handler)();
    }
}

impl<F: Fn()> Drop for MockIsrGuard<F> {
    fn drop(&mut self) {
        // Mock implementation: nothing to clean up
    }
}

impl<F: Fn()> IsrOps<F> for MockIsrGuard<F> {
    // `unsafe` only because the `IsrOps` trait's method signature requires
    // it (to match the real esp-hal backend's safety contract) — the mock
    // body itself performs no unsafe operation.
    #[allow(unsafe_code)]
    unsafe fn register(handler: F) -> Self {
        Self::new(handler)
    }
}