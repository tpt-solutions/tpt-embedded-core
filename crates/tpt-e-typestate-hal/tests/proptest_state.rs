#![cfg(feature = "mock")]
#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_typestate_hal::state::{Complete, Configured, Idle, Transferring};
use tpt_e_typestate_hal::mock::MockDmaChannel;

proptest! {
    #[test]
    fn idle_to_complete_sequence(channel_id: u8) {
        let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
        let idle: DmaChannel<Idle, MockDmaChannel> = DmaChannel::mock(channel_id);
        let configured: DmaChannel<Configured, MockDmaChannel> = idle.configure(buf, 64);
        let transferring: DmaChannel<Transferring, MockDmaChannel> = configured.start();
        let _complete: DmaChannel<Complete, MockDmaChannel> = transferring.wait();
    }

    #[test]
    fn multiple_sequential_transfers(channel_id: u8) {
        // Verify we can perform multiple sequential transfers on the same channel
        for _i in 0..3 {
            let buf: &'static mut [u8] = Box::leak(vec![0u8; 32].into_boxed_slice());
            let idle: DmaChannel<Idle, MockDmaChannel> = DmaChannel::mock(channel_id);
            let configured: DmaChannel<Configured, MockDmaChannel> = idle.configure(buf, 32);
            let transferring: DmaChannel<Transferring, MockDmaChannel> = configured.start();
            let _complete: DmaChannel<Complete, MockDmaChannel> = transferring.wait();
        }
    }

    #[test]
    fn different_channel_ids(channel_id in 0u8..8) {
        let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
        let idle: DmaChannel<Idle, MockDmaChannel> = DmaChannel::mock(channel_id);
        let configured: DmaChannel<Configured, MockDmaChannel> = idle.configure(buf, 64);
        let transferring: DmaChannel<Transferring, MockDmaChannel> = configured.start();
        let _complete: DmaChannel<Complete, MockDmaChannel> = transferring.wait();
    }

    #[test]
    fn different_buffer_sizes(channel_id: u8, size in 1usize..1024) {
        let buf: &'static mut [u8] = Box::leak(vec![0u8; size].into_boxed_slice());
        let idle: DmaChannel<Idle, MockDmaChannel> = DmaChannel::mock(channel_id);
        let configured: DmaChannel<Configured, MockDmaChannel> = idle.configure(buf, size);
        let transferring: DmaChannel<Transferring, MockDmaChannel> = configured.start();
        let _complete: DmaChannel<Complete, MockDmaChannel> = transferring.wait();
    }
}