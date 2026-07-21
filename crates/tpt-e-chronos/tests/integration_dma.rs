//! Integration test: `tpt-e-chronos` ring buffer fed via `tpt-e-typestate-hal` DMA handle.

#![allow(missing_docs)]

use tpt_e_chronos::ring_buf::RingBuf;
use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_typestate_hal::state::Complete;

/// Simulate a DMA transfer: fill the ring buffer, then hand off to DMA.
#[test]
fn ring_buffer_via_dma_handle() {
    let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
    let ring = RingBuf::<u8, 32>::new(0);

    // Push some telemetry data
    for i in 0u8..10 {
        assert!(ring.push(i).is_ok());
    }

    // Simulate DMA channel lifecycle using mock
    let dma = DmaChannel::mock(0);
    let dma = dma.configure(buf, 64);

    // At this point, in a real system, the DMA would be configured
    // to read from the ring buffer. For the mock, we just verify
    // the state transition compiles and works.
    let dma = dma.start();
    let _complete: DmaChannel<Complete> = dma.wait();

    // Verify ring buffer still has our data
    assert_eq!(ring.len(), 10);
    for i in 0u8..10 {
        assert_eq!(ring.pop(), Some(i));
    }
}