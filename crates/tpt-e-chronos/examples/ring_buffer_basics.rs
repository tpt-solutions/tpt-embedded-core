//! Minimal example: push/pop through a `RingBuf`, then lend it to a
//! (simulated) DMA transfer and reclaim it afterward.
//!
//! Run with:
//!
//! ```text
//! cargo run -p tpt-e-chronos --example ring_buffer_basics --features mock
//! ```

// `DmaLoan::reclaim` is intentionally `unsafe` (see its docs) — this
// example exercises it the same way the crate's own tests do.
#![allow(unsafe_code)]

use tpt_e_chronos::ring_buf::RingBuf;

fn main() {
    let mut telemetry: RingBuf<u32, 8> = RingBuf::new(0);

    for sample in [10, 20, 30, 40] {
        telemetry.push(sample).expect("buffer has room");
    }
    println!("buffered {} samples", telemetry.len());

    while let Some(sample) = telemetry.pop() {
        println!("read sample: {sample}");
    }

    // Lend the buffer to a (simulated) DMA transfer. While the loan is
    // outstanding, the compiler forbids calling push/pop on `telemetry`.
    telemetry.push(99).expect("buffer has room");
    let (loan, view) = telemetry.lend_for_dma();
    println!("DMA sees {} words in the backing buffer", view.len());

    // SAFETY: no real DMA transfer is in flight in this example.
    let telemetry = unsafe { loan.reclaim() };
    println!("reclaimed buffer, len = {}", telemetry.len());
}
