//! Minimal example: drive a `DmaChannel` through its full typestate
//! lifecycle (`Idle -> Configured -> Transferring -> Complete`) using the
//! mock backend.
//!
//! Run with:
//!
//! ```text
//! cargo run -p tpt-e-typestate-hal --example dma_transfer --features mock
//! ```

use tpt_e_typestate_hal::dma::DmaChannel;

fn main() {
    let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());

    let idle = DmaChannel::mock(0);
    println!("channel idle");

    let configured = idle.configure(buf, 64);
    println!("channel configured");

    let transferring = configured.start();
    println!("transfer started");

    let _complete = transferring.wait();
    println!("transfer complete");
}
