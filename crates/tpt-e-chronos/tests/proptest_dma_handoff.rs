//! Property tests for the DMA handoff path (`dma_handoff.rs`).

#![cfg(feature = "mock")]
#![allow(missing_docs)]
#![allow(unsafe_code)]

use proptest::prelude::*;
use tpt_e_chronos::ring_buf::RingBuf;

proptest! {
    #[test]
    fn lend_reclaim_preserves_fifo_order(
        values: Vec<u32>,
    ) {
        let mut ring = RingBuf::<u32, 16>::new(0);
        for &v in &values {
            let _ = ring.push(v);
        }

        let (loan, view) = ring.lend_for_dma();
        // View must reflect the current backing storage
        prop_assert_eq!(view.len(), 16);

        // SAFETY: no real DMA transfer in this test.
        let ring = unsafe { loan.reclaim() };

        // FIFO order must be preserved: pop yields values in push order.
        let mut popped = Vec::new();
        while let Some(v) = ring.pop() {
            popped.push(v);
        }
        let expected: Vec<u32> = values.into_iter().take(16).collect();
        prop_assert_eq!(popped, expected);
    }

    #[test]
    fn loaned_view_matches_backing_storage(
        init_val: u32,
        pushed: Vec<u32>,
    ) {
        let mut ring = RingBuf::<u32, 4>::new(init_val);
        for &v in pushed.iter().take(4) {
            let _ = ring.push(v);
        }

        let (loan, view) = ring.lend_for_dma();
        // Slots beyond what was pushed should contain init_val
        let pushed_count = pushed.iter().take(4).count();
        for (i, &slot) in view.iter().enumerate() {
            if i < pushed_count {
                prop_assert_eq!(slot, pushed[i]);
            } else {
                prop_assert_eq!(slot, init_val);
            }
        }

        let _ = unsafe { loan.reclaim() };
    }

    #[test]
    fn multiple_lend_reclaim_cycles_preserve_data(
        round_1: Vec<u32>,
        round_2: Vec<u32>,
    ) {
        let mut ring = RingBuf::<u32, 8>::new(0);

        // First round: push, lend, reclaim, pop all
        for &v in &round_1 {
            let _ = ring.push(v);
        }
        let (loan, _view) = ring.lend_for_dma();
        let ring = unsafe { loan.reclaim() };
        while ring.pop().is_some() {}

        // Second round: push, lend, reclaim, verify
        for &v in &round_2 {
            let _ = ring.push(v);
        }
        let (loan, _view) = ring.lend_for_dma();
        let ring = unsafe { loan.reclaim() };

        let mut popped = Vec::new();
        while let Some(v) = ring.pop() {
            popped.push(v);
        }
        let expected: Vec<u32> = round_2.into_iter().take(8).collect();
        prop_assert_eq!(popped, expected);
    }
}
