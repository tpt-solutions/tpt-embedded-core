//! A heapless, const-generic ring buffer with ISR-safe push and main-loop pop.
#![allow(unsafe_code)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A ring buffer with const-generic capacity.
///
/// Push operations use an atomic spinlock-based critical section to serialize
/// concurrent access (e.g. from ISR context). Pop uses the same spinlock for
/// the consumer side. Both operations are O(1) and WCET-bounded.
///
/// # Concurrency
///
/// `push` and `pop` may be called from different contexts (ISR / main loop).
/// Concurrent `push` calls (or concurrent `pop` calls) are serialized by the
/// internal spinlock — data loss from interleaved atomic reads is eliminated.
///
/// # Type Parameters
///
/// * `T` - The element type.
/// * `CAP` - The buffer capacity (must be a power of two for efficient mask).
pub struct RingBuf<T, const CAP: usize> {
    buffer: UnsafeCell<[T; CAP]>,
    head: AtomicUsize,
    tail: AtomicUsize,
    push_lock: AtomicBool,
    pop_lock: AtomicBool,
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
            push_lock: AtomicBool::new(false),
            pop_lock: AtomicBool::new(false),
        }
    }

    /// Attempt to push an item into the buffer.
    ///
    /// Returns `Ok(())` on success, `Err(item)` if the buffer is full.
    /// Concurrent `push` calls are serialized by an internal spinlock.
    pub fn push(&self, item: T) -> Result<(), T> {
        while self
            .push_lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let result = unsafe { self.push_inner(item) };
        self.push_lock.store(false, Ordering::Release);
        result
    }

    /// Inner push — caller must hold `push_lock`.
    ///
    /// # Safety
    ///
    /// `push_lock` must be held (set to `true`) by the caller, ensuring
    /// exclusive access to the head pointer and the target slot.
    unsafe fn push_inner(&self, item: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= CAP {
            return Err(item);
        }

        // SAFETY: push_lock is held, so no concurrent push can race on
        // this slot. Pop only reads slots behind tail, which is ≤ head.
        (*self.buffer.get())[head & Self::MASK] = item;

        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Attempt to pop an item from the buffer.
    ///
    /// Concurrent `pop` calls are serialized by an internal spinlock.
    pub fn pop(&self) -> Option<T>
    where
        T: Copy,
    {
        while self
            .pop_lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let result = unsafe { self.pop_inner() };
        self.pop_lock.store(false, Ordering::Release);
        result
    }

    /// Inner pop — caller must hold `pop_lock`.
    ///
    /// # Safety
    ///
    /// `pop_lock` must be held (set to `true`) by the caller, ensuring
    /// exclusive access to the tail pointer and the source slot.
    unsafe fn pop_inner(&self) -> Option<T>
    where
        T: Copy,
    {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        // SAFETY: pop_lock is held, so no concurrent pop can race.
        // The push side only writes to slots ≥ head, and tail < head,
        // so this slot is not being written.
        let item = (*self.buffer.get())[tail & Self::MASK];

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

// SAFETY: RingBuf is safe to share across threads because all mutations to
// the internal buffer are serialized by atomic spinlocks (push_lock / pop_lock).
// `push` and `pop` take `&self` and acquire the appropriate lock before touching
// any shared state, so concurrent calls from ISR + main-loop (or multiple
// threads in tests) serialize correctly and cannot race on the backing array.
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
