//! Zero-copy DMA handoff path for the ring buffer.
//!
//! Allows a [`RingBuf`] to lend its internal buffer to a DMA transfer,
//! then reclaim it once the transfer completes.
//!
//! # Cross-crate integration
//!
//! When the optional `tpt-e-typestate-hal` dependency is enabled,
//! [`transfer_with_dma`] provides a convenience function that performs
//! the lend → DMA transfer → reclaim cycle in one call, using a
//! `tpt-e-typestate-hal` `DmaChannel<Transferring>` to drive the transfer.
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

#![allow(unsafe_code)]

use core::fmt;
use crate::ring_buf::RingBuf;

#[cfg(feature = "tpt-e-typestate-hal")]
use tpt_e_typestate_hal::dma::DmaChannel;
#[cfg(feature = "tpt-e-typestate-hal")]
use tpt_e_typestate_hal::state::{Complete, Transferring};
#[cfg(all(feature = "tpt-e-typestate-hal", feature = "mock"))]
use tpt_e_typestate_hal::mock::MockDmaChannel;

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

/// Complete a DMA transfer using a `RingBuf`'s backing storage as the
/// transfer buffer, then reclaim the buffer for normal `push`/`pop` access.
///
/// This is the primary integration point between `tpt-e-chronos` and
/// `tpt-e-typestate-hal`: the ring buffer lends its internal storage to a
/// DMA transfer, the transfer completes, and the buffer is reclaimed — all
/// without copying.
///
/// # Protocol
///
/// 1. The ring buffer's backing storage is lent to the DMA transfer via
///    [`RingBuf::lend_for_dma`].
/// 2. The DMA transfer is performed using the provided channel (the caller
///    must ensure the buffer is compatible with the channel's transfer
///    width).
/// 3. The buffer is reclaimed via [`DmaLoan::reclaim`], restoring normal
///    `push`/`pop` access.
///
/// # Safety
///
/// The DMA transfer must have fully completed before this function is
/// called. The caller must ensure the buffer's element type `T` is
/// compatible with the DMA transfer width (typically `u8`, `u16`, or `u32`).
///
/// # Example (mock)
///
/// ```rust
/// #![allow(unsafe_code)]
/// use tpt_e_chronos::ring_buf::RingBuf;
/// use tpt_e_chronos::dma_handoff::transfer_with_dma;
/// use tpt_e_typestate_hal::dma::DmaChannel;
///
/// let mut ring = RingBuf::<u32, 4>::new(0);
/// let _ = ring.push(42);
///
/// // Drive the channel through Idle → Configured → Transferring
/// static mut DMA_BUF: [u8; 16] = [0u8; 16];
/// let channel = DmaChannel::<_, tpt_e_typestate_hal::mock::MockDmaChannel>::mock(0);
/// let channel = unsafe { channel.configure(&mut DMA_BUF, 16) };
/// let channel = channel.start();
/// let mut ring = unsafe { transfer_with_dma(&mut ring, channel) };
/// assert_eq!(ring.pop(), Some(42));
/// ```
#[cfg(feature = "tpt-e-typestate-hal")]
pub unsafe fn transfer_with_dma<T: Copy, const CAP: usize, B>(
    ring: &mut RingBuf<T, CAP>,
    channel: DmaChannel<Transferring, B>,
) -> &mut RingBuf<T, CAP>
where
    B: tpt_e_typestate_hal::backend::TransferringOps<Complete = B>,
{
    let (loan, _view) = ring.lend_for_dma();
    let _complete = channel.wait();
    loan.reclaim()
}
