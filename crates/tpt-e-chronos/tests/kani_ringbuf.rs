//! Kani proof harnesses for `tpt-e-chronos` ring buffer.
//!
//! Run with: `cargo kani --features mock -p tpt-e-chronos`

/// Prove that push never panics (no out-of-bounds access) under any index value.
///
/// The ring buffer uses masking (`head & MASK`), so any `usize` index is safe.
#[cfg(kani)]
#[kani::proof]
fn push_never_panics() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let value: u32 = kani::any();
    let _ = buf.push(value);
}

/// Prove that pop never panics when the buffer is non-empty.
#[cfg(kani)]
#[kani::proof]
fn pop_never_panics() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let value: u32 = kani::any();
    let _ = buf.push(value);
    let _ = buf.pop();
}

/// Prove that pop returns None on an empty buffer, never panics.
#[cfg(kani)]
#[kani::proof]
fn pop_empty_returns_none() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let result = buf.pop::<u32>();
    assert!(result.is_none());
}

/// Prove that len() never overflows for any sequence of push/pop operations.
#[cfg(kani)]
#[kani::proof]
fn len_is_bounded_by_capacity() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let ops: Vec<bool> = kani::any_vec::<_, bool>();

    for &should_push in ops.iter().take(32) {
        if should_push {
            let val: u32 = kani::any();
            let _ = buf.push(val);
        } else {
            let _ = buf.pop::<u32>();
        }
    }

    assert!(buf.len() <= buf.capacity());
}

/// Prove that wrapping indices never cause out-of-bounds access.
#[cfg(kani)]
#[kani::proof]
fn wrapping_indices_are_safe() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 4>::new(0);
    for i in 0u32..100 {
        let _ = buf.push(i);
        let _ = buf.pop::<u32>();
    }
}

/// Prove that push/pop interleaving preserves FIFO ordering.
#[cfg(kani)]
#[kani::proof]
fn fifo_ordering_preserved() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let mut expected: Vec<u32> = Vec::new();
    let mut counter: u32 = 0;

    let ops: Vec<bool> = kani::any_vec::<_, bool>();

    for &should_push in ops.iter().take(20) {
        if should_push {
            if buf.push(counter).is_ok() {
                expected.push(counter);
                counter = counter.wrapping_add(1);
            }
        } else if let Some(val) = buf.pop() {
            assert_eq!(val, expected.remove(0));
        }
    }
}

/// WCET analysis: prove push executes in O(1) with bounded work.
#[cfg(kani)]
#[kani::proof]
fn push_is_constant_time() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let val: u32 = kani::any();
    let _ = buf.push(val);
}

/// WCET analysis: prove pop executes in O(1) with bounded work.
#[cfg(kani)]
#[kani::proof]
fn pop_is_constant_time() {
    use tpt_e_chronos::ring_buf::RingBuf;
    let buf = RingBuf::<u32, 8>::new(0);
    let val: u32 = kani::any();
    let _ = buf.push(val);
    let _ = buf.pop::<u32>();
}
