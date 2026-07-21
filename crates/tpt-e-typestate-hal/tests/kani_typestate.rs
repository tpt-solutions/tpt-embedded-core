//! Kani proof harnesses for `tpt-e-typestate-hal` typestate transitions.
//!
//! Run with: `cargo kani --features mock -p tpt-e-typestate-hal`

/// Prove that the complete DMA lifecycle (Idle -> Configured -> Transferring -> Complete)
/// can always be executed without panics.
#[cfg(kani)]
#[kani::proof]
fn dma_lifecycle_never_panics() {
    use tpt_e_typestate_hal::dma::DmaChannel;
    use tpt_e_typestate_hal::mock::MockDmaChannel;

    let channel_id: u8 = kani::any();
    let len: usize = kani::any();
    kani::assume(len > 0 && len <= 4096);

    let buf: &'static mut [u8] = Box::leak(vec![0u8; len].into_boxed_slice());
    let idle = DmaChannel::<_, MockDmaChannel>::mock(channel_id);
    let configured = idle.configure(buf, len);
    let transferring = configured.start();
    let _complete = transferring.wait();
}

/// Prove that MockDmaChannel::configure always returns Self (no panics).
#[cfg(kani)]
#[kani::proof]
fn mock_configure_always_succeeds() {
    use tpt_e_typestate_hal::mock::MockDmaChannel;
    use tpt_e_typestate_hal::backend::IdleOps;

    let channel_id: u8 = kani::any();
    let channel = MockDmaChannel::new(channel_id);
    let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
    let _ = channel.configure(buf, 64);
}

/// Prove that MockDmaChannel::start always returns Self (no panics).
#[cfg(kani)]
#[kani::proof]
fn mock_start_always_succeeds() {
    use tpt_e_typestate_hal::mock::MockDmaChannel;
    use tpt_e_typestate_hal::backend::{IdleOps, ConfiguredOps};

    let channel_id: u8 = kani::any();
    let channel = MockDmaChannel::new(channel_id);
    let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
    let configured = channel.configure(buf, 64);
    let _ = configured.start();
}

/// Prove that MockDmaChannel::wait always returns Self (no panics).
#[cfg(kani)]
#[kani::proof]
fn mock_wait_always_succeeds() {
    use tpt_e_typestate_hal::mock::MockDmaChannel;
    use tpt_e_typestate_hal::backend::{IdleOps, ConfiguredOps, TransferringOps};

    let channel_id: u8 = kani::any();
    let channel = MockDmaChannel::new(channel_id);
    let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
    let configured = channel.configure(buf, 64);
    let transferring = configured.start();
    let _ = transferring.wait();
}
