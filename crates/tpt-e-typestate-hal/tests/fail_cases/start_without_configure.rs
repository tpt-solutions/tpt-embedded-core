use tpt_e_typestate_hal::dma::DmaChannel;
use tpt_e_typestate_hal::mock::MockDmaChannel;

fn main() {
    let channel = DmaChannel::new(MockDmaChannel::new(0));
    // ERROR: `start()` is not available on `DmaChannel<Idle>` — must configure first.
    let _ = channel.start();
}
