//! Typestate markers for the DMA/ISR state machine.
//!
//! Transitions are enforced at compile time — an invalid call (e.g., starting
//! a transfer without configuration) produces a compile error, not a runtime panic.

/// The peripheral is powered on but not configured.
#[derive(Debug, Copy, Clone)]
pub struct Idle;

/// The peripheral has been configured and is ready to start a transfer.
#[derive(Debug, Copy, Clone)]
pub struct Configured;

/// A DMA transfer is actively in progress. The buffer is exclusively borrowed.
#[derive(Debug, Copy, Clone)]
pub struct Transferring;

/// The DMA transfer has completed. The buffer is accessible to the main thread again.
#[derive(Debug, Copy, Clone)]
pub struct Complete;
