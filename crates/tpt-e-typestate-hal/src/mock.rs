//! Mock implementations for host-side testing.

/// A mock DMA channel for host-side testing.
#[derive(Debug, Copy, Clone)]
pub struct MockDmaChannel;

impl MockDmaChannel {
    /// Create a new mock DMA channel.
    pub fn new() -> Self {
        Self
    }
}
