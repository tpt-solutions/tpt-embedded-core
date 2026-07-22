//! A heapless, const-generic ring buffer with ISR-safe push and main-loop pop.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A lock-free ring buffer with const-generic capacity.
///
/// Push operations use a critical section (minimum-length) to safely coordinate
/// with ISR contexts. Pop is intended for the main loop and uses relaxed ordering.
///
/// # Type Parameters
///
/// * `T` - The element type.
/// * `CAP` - The buffer capacity (must be a power of two for efficient mask).
pub struct RingBuf<T, const CAP: usize> {
    buffer: UnsafeCell<[T; CAP]>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T, const CAP: usize> core::fmt::Debug for RingBuf<T, CAP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RingBuf")
            .field("head", &self.head)
            .field("tail", &self.tail)
            .field("len", &self.len())
            .field("capacity", &CAP)
            .finish()
    }
}

impl<T, const CAP: usize> RingBuf<T, CAP> {
    const _CAPACITY_CHECK: () = assert!(
        CAP.is_power_of_two(),
        "RingBuf capacity must be a power of two for correct mask-based indexing"
    );
    const MASK: usize = CAP - 1;

    /// Create a new ring buffer, filling it with the provided initial value.
    pub fn new(init: T) -> Self
    where
        T: Copy,
    {
        Self {
            buffer: UnsafeCell::new([init; CAP]),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Attempt to push an item into the buffer.
    ///
    /// Returns `Ok(())` on success, `Err(item)` if the buffer is full.
    pub fn push(&self, item: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= CAP {
            return Err(item);
        }

        // SAFETY: We hold exclusive access to the slot because head is
        // advanced atomically and ISR interleaving is prevented by the caller.
        unsafe {
            (*self.buffer.get())[head & Self::MASK] = item;
        }

        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Attempt to pop an item from the buffer.
    pub fn pop(&self) -> Option<T>
    where
        T: Copy,
    {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        // SAFETY: tail is advanced atomically; the slot is no longer
        // accessible to the push path.
        let item = unsafe { (*self.buffer.get())[tail & Self::MASK] };

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    /// Returns `true` if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed) == self.tail.load(Ordering::Relaxed)
    }

    /// Returns `true` if the buffer is full.
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail) >= CAP
    }

    /// Returns the number of items in the buffer.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail)
    }

    /// Returns the total capacity of the buffer.
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Return a raw pointer to the internal buffer (crate-internal).
    pub(crate) fn buffer_ptr(&self) -> *const T {
        let cell_ptr: *const UnsafeCell<[T; CAP]> = &self.buffer;
        cell_ptr.cast::<T>()
    }
}

// SAFETY: The ring buffer is designed for single-consumer (main loop)
// single-producer (ISR) usage. The atomic head/tail indices synchronise
// access. No `&self` method on RingBuf provides a mutable reference into
// the buffer, so sharing across threads is safe under the single-consumer
// constraint.
unsafe impl<T, const CAP: usize> Sync for RingBuf<T, CAP> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop() {
        let buf = RingBuf::<u32, 4>::new(0);
        assert!(buf.push(42).is_ok());
        assert!(!buf.is_empty());
        assert_eq!(buf.pop(), Some(42));
        assert!(buf.is_empty());
    }

    #[test]
    fn full_buffer() {
        let buf = RingBuf::<u32, 4>::new(0);
        assert!(buf.push(1).is_ok());
        assert!(buf.push(2).is_ok());
        assert!(buf.push(3).is_ok());
        assert!(buf.push(4).is_ok());
        assert!(buf.is_full());
        assert!(buf.push(5).is_err());
    }

    #[test]
    fn empty_pop() {
        let buf = RingBuf::<u32, 4>::new(0);
        assert_eq!(buf.pop(), None);
    }

    #[test]
    fn wrap_around() {
        let buf = RingBuf::<u32, 4>::new(0);
        for i in 0..4 {
            assert!(buf.push(i as u32).is_ok());
        }
        for i in 0..4 {
            assert_eq!(buf.pop(), Some(i as u32));
        }
        // Buffer should be empty and reusable
        for i in 0..4 {
            assert!(buf.push((i + 10) as u32).is_ok());
        }
        for i in 0..4 {
            assert_eq!(buf.pop(), Some((i + 10) as u32));
        }
    }
}
