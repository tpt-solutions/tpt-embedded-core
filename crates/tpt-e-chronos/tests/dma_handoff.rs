//! Tests for the zero-copy DMA handoff path (`dma_handoff.rs`).

#![allow(missing_docs)]
// `DmaLoan::reclaim` is intentionally `unsafe` (see its docs); exercising
// it from a test is the same exception the crate root grants itself.
#![allow(unsafe_code)]

use tpt_e_chronos::ring_buf::RingBuf;

/// Lending, reading through the loan, and reclaiming preserves the data
/// and restores normal push/pop access afterward.
#[test]
fn lend_reclaim_round_trip() {
    let mut ring = RingBuf::<u32, 4>::new(0);
    assert!(ring.push(1).is_ok());
    assert!(ring.push(2).is_ok());

    let (loan, view) = ring.lend_for_dma();
    assert_eq!(view.len(), 4);

    // SAFETY: no DMA transfer is actually in flight in this test; we're
    // just verifying the reclaim path restores access.
    let ring = unsafe { loan.reclaim() };

    assert_eq!(ring.pop(), Some(1));
    assert_eq!(ring.pop(), Some(2));
    assert_eq!(ring.pop(), None);
}

/// The loaned view sees exactly the current backing storage contents,
/// including uninitialized/default slots beyond what's been pushed.
#[test]
fn loaned_view_reflects_backing_storage() {
    let mut ring = RingBuf::<u32, 4>::new(0xAAAA_AAAA);
    assert!(ring.push(1).is_ok());

    let (loan, view) = ring.lend_for_dma();
    assert_eq!(view[0], 1);
    assert_eq!(view[1], 0xAAAA_AAAA);

    // SAFETY: no DMA transfer in flight; just reclaiming to end the test.
    let _ = unsafe { loan.reclaim() };
}
