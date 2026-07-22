use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_typestate_hal::mock::MockDmaChannel;

fn main() {
    static mut BUF: [u8; 64] = [0u8; 64];
    let channel = DmaChannel::new(MockDmaChannel::new(0))
        .configure(unsafe { &mut BUF }, 64)
        .start()
        .wait();
    // ERROR: `start()` is not available on `DmaChannel<Complete>` — already done.
    let _ = channel.start();
}
