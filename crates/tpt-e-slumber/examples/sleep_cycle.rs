//! Minimal example: collect the three proof tokens `enter_deep_sleep`
//! requires, showing the compile-time gating that is this crate's whole
//! point — miss a token and the call to `enter_deep_sleep` fails to
//! compile, not at runtime.
//!
//! Run with:
//!
//! ```text
//! cargo run -p tpt-e-slumber --example sleep_cycle --features mock
//! ```

use tpt_e_slumber::sleep::SleepController;
use tpt_e_slumber::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

fn main() {
    let controller = SleepController::new();

    // In production, each token is minted by the subsystem that proved its
    // precondition (DMA idle, RTC isolated, buffers flushed) — the public
    // constructors are `pub(crate)` for exactly that reason. `Token::mock()`
    // exists only under the `mock` feature, for host-side demonstration
    // and testing like this.
    let dma_token = DmaParkedToken::mock();
    let rtc_token = RtcIsolatedToken::mock();
    let buffers_token = BuffersFlushedToken::mock();

    println!("all three proof tokens collected; ready to enter deep sleep");

    // `enter_deep_sleep` returns `!` (it never returns), so it's commented
    // out here to keep this example runnable to completion:
    // controller.enter_deep_sleep(dma_token, rtc_token, buffers_token);
    let _ = (controller, dma_token, rtc_token, buffers_token);
}
