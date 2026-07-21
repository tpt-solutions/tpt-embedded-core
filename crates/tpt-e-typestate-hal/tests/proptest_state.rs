#![cfg(feature = "mock")]
#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_typestate_hal::state::{Complete, Configured, Idle, Transferring};

proptest! {
    #[test]
    fn idle_to_complete_sequence(channel_id: u8) {
        let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
        let idle: DmaChannel<Idle> = DmaChannel::new(channel_id);
        let configured: DmaChannel<Configured> = idle.configure(buf, 64);
        let transferring: DmaChannel<Transferring> = configured.start();
        let _complete: DmaChannel<Complete> = transferring.wait();
    }
}
