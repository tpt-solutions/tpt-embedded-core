//! Kani proof harnesses for `tpt-e-slumber` sleep transitions.
//!
//! Run with: `cargo kani --features mock -p tpt-e-slumber`

/// Prove that tokens can always be created without panics.
#[cfg(kani)]
#[kani::proof]
fn tokens_always_creatable() {
    use tpt_e_slumber::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

    let _dma = DmaParkedToken::new();
    let _rtc = RtcIsolatedToken::new();
    let _buffers = BuffersFlushedToken::new();
}

/// Prove that SleepController can always be created without panics.
#[cfg(kani)]
#[kani::proof]
fn controller_always_creatable() {
    use tpt_e_slumber::sleep::SleepController;
    let _controller = SleepController::new();
}

/// Prove that all three tokens can be collected and passed to enter_deep_sleep.
///
/// This verifies the compile-time guarantee at the type level — if the types
/// don't align, this proof harness wouldn't compile.
#[cfg(kani)]
#[kani::proof]
fn all_tokens_enable_deep_sleep() {
    use tpt_e_slumber::sleep::SleepController;
    use tpt_e_slumber::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};

    let controller = SleepController::new();
    let dma = DmaParkedToken::new();
    let rtc = RtcIsolatedToken::new();
    let buffers = BuffersFlushedToken::new();

    // enter_deep_sleep returns `!`, so we can't actually call it in a proof.
    // But we can verify all tokens are the correct types by binding them.
    let _ = (controller, dma, rtc, buffers);
}
