//! Zero-copy DMA handoff path for the ring buffer.
//!
//! Allows a `RingBuf` to lend its internal buffer to a DMA transfer,
//! then reclaim it once the transfer completes.

use core::fmt;
use crate::ring_buf::RingBuf;

/// A handle representing a buffer lent to a DMA transfer.
///
/// When this handle is dropped without being reclaimed, the buffer
/// is considered lost. The owning `RingBuf` must not be used until
/// the buffer is reclaimed.
pub struct DmaLoan<'a, T, const CAP: usize> {
    buf: &'a RingBuf<T, CAP>,
}

impl<'a, T: fmt::Debug, const CAP: usize> fmt::Debug for DmaLoan<'a, T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DmaLoan").field("buf", &self.buf).finish()
    }
}

impl<'a, T: Copy, const CAP: usize> RingBuf<T, CAP> {
    /// Lend the internal buffer to a DMA transfer.
    ///
    /// Returns a loan token and a pointer to the buffer. The loan
    /// must be reclaimed once the DMA transfer completes.
    pub fn lend_for_dma(&'a self) -> (DmaLoan<'a, T, CAP>, &'a [T]) {
        let ptr: *const T = self.buffer_ptr();
        // SAFETY: The ring buffer is not modified during the DMA transfer.
        // The caller must ensure the DMA transfer completes before dropping
        // the loan or accessing the buffer.
        let slice = unsafe { core::slice::from_raw_parts(ptr, CAP) };
        (DmaLoan { buf: self }, slice)
    }
}

impl<T, const CAP: usize> RingBuf<T, CAP> {
    /// Return a raw pointer to the internal buffer.
    fn buffer_ptr(&self) -> *const T {
        let cell_ptr = &self.buffer as *const _;
        cell_ptr.cast::<T>()
    }
}

impl<'a, T, const CAP: usize> DmaLoan<'a, T, CAP> {
    /// Reclaim the buffer after DMA completes.
    ///
    /// # Safety
    ///
    /// The DMA transfer must have fully completed before calling this.
    /// The caller must ensure no further DMA accesses to the buffer occur.
    pub unsafe fn reclaim(self) -> &'a RingBuf<T, CAP> {
        self.buf
    }
}
