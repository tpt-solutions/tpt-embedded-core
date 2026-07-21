//! Safety proof tokens for sleep transitions.

/// Token proving DMA has been safely parked.
#[derive(Debug, Copy, Clone)]
pub struct DmaParkedToken;

/// Token proving RTC memory has been isolated.
#[derive(Debug, Copy, Clone)]
pub struct RtcIsolatedToken;
