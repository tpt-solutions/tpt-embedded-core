//! Real multi-threaded concurrency stress test for `RingBuf`.
//!
//! The proptests in `proptest_ringbuf.rs` only exercise *interleavings* of
//! push/pop calls from a single thread — they never actually run pushes
//! concurrently. That's exactly the gap that let the original data-loss bug
//! (documented in the root `todo.md`: 8 threads pushing to a `CAP=65536`
//! buffer silently lost 95k+ of 160k items) go undetected until it was
//! manually reproduced ad hoc. This test makes that repro permanent.

#![allow(missing_docs)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use tpt_e_chronos::ring_buf::RingBuf;

#[test]
fn concurrent_pushes_lose_no_data() {
    // Nothing drains concurrently here, so CAP must exceed the total item
    // count (THREADS * PER_THREAD) or pushes legitimately start failing
    // once the buffer fills — that would be a test bug, not a RingBuf bug.
    // Kept modest (rather than matching the original 65536-capacity bug
    // repro) because `RingBuf::new` builds the backing array on the stack
    // before moving it behind `Arc`, and a large `CAP` overflows the
    // default thread stack.
    const CAP: usize = 8192;
    const THREADS: usize = 8;
    const PER_THREAD: usize = 800;

    let buf = Arc::new(RingBuf::<u32, CAP>::new(0));
    let pushed = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let buf = Arc::clone(&buf);
            let pushed = Arc::clone(&pushed);
            thread::spawn(move || {
                for i in 0..PER_THREAD {
                    if buf.push(i as u32).is_ok() {
                        let _ = pushed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("pusher thread panicked");
    }

    let expected = pushed.load(Ordering::Relaxed);
    assert_eq!(
        expected,
        THREADS * PER_THREAD,
        "every push should succeed: CAP ({CAP}) comfortably exceeds total items ({})",
        THREADS * PER_THREAD
    );

    let mut popped = 0;
    while buf.pop().is_some() {
        popped += 1;
    }

    assert_eq!(
        popped, expected,
        "every item reported as successfully pushed must be poppable — a mismatch means \
         concurrent pushes corrupted or lost data"
    );
}

#[test]
fn concurrent_push_and_pop_never_panics_or_corrupts() {
    const CAP: usize = 1024;
    const ITEMS: usize = 50_000;

    let buf = Arc::new(RingBuf::<u32, CAP>::new(0));
    let produced = Arc::new(AtomicUsize::new(0));
    let consumed = Arc::new(AtomicUsize::new(0));

    let producer = {
        let buf = Arc::clone(&buf);
        let produced = Arc::clone(&produced);
        thread::spawn(move || {
            let mut i = 0u32;
            while (i as usize) < ITEMS {
                if buf.push(i).is_ok() {
                    let _ = produced.fetch_add(1, Ordering::Relaxed);
                    i += 1;
                }
            }
        })
    };

    let consumer = {
        let buf = Arc::clone(&buf);
        let consumed = Arc::clone(&consumed);
        thread::spawn(move || {
            let mut last: Option<u32> = None;
            let mut n = 0usize;
            while n < ITEMS {
                if let Some(v) = buf.pop() {
                    if let Some(prev) = last {
                        assert!(
                            v == prev.wrapping_add(1),
                            "pop() returned out-of-order/corrupted data: prev={prev} got={v}"
                        );
                    }
                    last = Some(v);
                    let _ = consumed.fetch_add(1, Ordering::Relaxed);
                    n += 1;
                }
            }
        })
    };

    producer.join().expect("producer thread panicked");
    consumer.join().expect("consumer thread panicked");

    assert_eq!(produced.load(Ordering::Relaxed), ITEMS);
    assert_eq!(consumed.load(Ordering::Relaxed), ITEMS);
}
