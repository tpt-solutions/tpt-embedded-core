use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_typestate_hal::mock::MockDmaChannel;

fn main() {
    static mut BUF: [u8; 64] = [0u8; 64];
    let channel = DmaChannel::new(MockDmaChannel::new(0))
        .configure(unsafe { &mut BUF }, 64);
    // ERROR: `wait()` is not available on `DmaChannel<Configured>` — must start first.
    let _ = channel.wait();
}
