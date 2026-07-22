//! Zero-copy DMA handoff path for the ring buffer.
//!
//! Allows a `RingBuf` to lend its internal buffer to a DMA transfer,
//! then reclaim it once the transfer completes.
//!
//! # Compile-time Exclusivity
//!
//! While a [`DmaLoan`] is outstanding, the compiler forbids calling
//! `push`/`pop` on the loaned `RingBuf` — this is enforced by the borrow
//! checker, not just documented:
//!
//! ```compile_fail
//! use tpt_e_chronos::ring_buf::RingBuf;
//!
//! let mut ring = RingBuf::<u32, 4>::new(0);
//! let (loan, _view) = ring.lend_for_dma();
//! let _ = ring.push(1); // ERROR: `ring` is exclusively borrowed by `loan`
//! let _ = loan;
//! ```

use core::fmt;
use crate::ring_buf::RingBuf;

/// A handle representing a buffer lent to a DMA transfer.
///
/// `lend_for_dma` takes the `RingBuf` by exclusive (`&mut`) reference, and
/// `DmaLoan` holds onto that exclusive borrow for its lifetime. This means
/// the compiler — not just documentation — prevents any `push`/`pop` call
/// on the owning `RingBuf` while a loan is outstanding: `push`/`pop` take
/// `&self`, and no shared borrow of the buffer can coexist with the `&mut`
/// borrow held by the loan. When this handle is dropped without being
/// reclaimed, the buffer is considered lost (the exclusive borrow is simply
/// never released back to a usable form).
pub struct DmaLoan<'a, T, const CAP: usize> {
    buf: &'a mut RingBuf<T, CAP>,
}

impl<'a, T: fmt::Debug, const CAP: usize> fmt::Debug for DmaLoan<'a, T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DmaLoan").field("buf", &self.buf).finish()
    }
}

impl<'a, T: Copy, const CAP: usize> RingBuf<T, CAP> {
    /// Lend the internal buffer to a DMA transfer.
    ///
    /// Returns a loan token and a read-only view of the buffer. The loan
    /// must be reclaimed once the DMA transfer completes. For as long as
    /// the returned `DmaLoan` is alive, the compiler forbids calling
    /// `push`/`pop` on this `RingBuf` (they require a shared borrow, which
    /// cannot coexist with the exclusive borrow the loan holds).
    pub fn lend_for_dma(&'a mut self) -> (DmaLoan<'a, T, CAP>, &'a [T]) {
        let ptr: *const T = self.buffer_ptr();
        // SAFETY: The exclusive borrow held by the returned `DmaLoan`
        // statically prevents any other access to `self` (including
        // `push`/`pop`) until the loan is reclaimed, so the DMA's read-only
        // view constructed here cannot race a concurrent write.
        let slice = unsafe { core::slice::from_raw_parts(ptr, CAP) };
        (DmaLoan { buf: self }, slice)
    }
}

impl<'a, T, const CAP: usize> DmaLoan<'a, T, CAP> {
    /// Reclaim the buffer after DMA completes, restoring normal
    /// `push`/`pop` access.
    ///
    /// # Safety
    ///
    /// The DMA transfer must have fully completed before calling this.
    /// The caller must ensure no further DMA accesses to the buffer occur.
    pub unsafe fn reclaim(self) -> &'a mut RingBuf<T, CAP> {
        self.buf
    }
}
