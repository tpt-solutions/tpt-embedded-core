//! Safe DMA handle API consumed by `tpt-e-chronos` and `tpt-e-cipher`.

use crate::state::{Complete, Configured, Idle, Transferring};

/// A DMA channel parameterised by its current typestate.
///
/// Transitions consume the handle and return a new handle in the next state:
///
/// - `DmaChannel<Idle>`
///   → `configure(...)` → `DmaChannel<Configured>`
///   → `start(...)`    → `DmaChannel<Transferring>`
///   → `wait(...)`     → `DmaChannel<Complete>`
#[derive(Debug)]
pub struct DmaChannel<State> {
    _state: core::marker::PhantomData<State>,
    channel_id: u8,
}

impl DmaChannel<Idle> {
    /// Create a new idle DMA channel (obtained from the HAL).
    pub fn new(channel_id: u8) -> Self {
        Self { _state: core::marker::PhantomData, channel_id }
    }

    /// Configure the DMA channel with a buffer and transfer parameters.
    pub fn configure(self, _buf: &'static mut [u8], _len: usize) -> DmaChannel<Configured> {
        DmaChannel { _state: core::marker::PhantomData, channel_id: self.channel_id }
    }
}

impl DmaChannel<Configured> {
    /// Start the DMA transfer.
    pub fn start(self) -> DmaChannel<Transferring> {
        DmaChannel { _state: core::marker::PhantomData, channel_id: self.channel_id }
    }
}

impl DmaChannel<Transferring> {
    /// Wait for the DMA transfer to complete, returning access to the buffer.
    pub fn wait(self) -> DmaChannel<Complete> {
        DmaChannel { _state: core::marker::PhantomData, channel_id: self.channel_id }
    }
}
