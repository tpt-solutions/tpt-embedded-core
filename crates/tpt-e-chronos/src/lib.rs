#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-chronos`
//!
//! A WCET-bounded, zero-allocation ring buffer for time-series telemetry data.
//!
//! The ring buffer uses a const-generic capacity and supports ISR-safe push (via
//! critical sections) and main-loop pop operations. It provides a zero-copy
//! handoff path to DMA via the `tpt-e-typestate-hal` safe handles.
//!
//! # Design Guarantees
//!
//! - **Zero-allocation**: Uses a fixed-size, const-generic array. No heap usage.
//! - **WCET-bounded**: `push()` and `pop()` execute in O(1) time with no
//!   data-dependent branching beyond a single comparison.
//! - **ISR-safe**: Push operations use atomic head/tail indices with acquire/release
//!   ordering to safely coordinate between ISR (producer) and main-loop (consumer).
//! - **FIFO-ordered**: Elements are always dequeued in the order they were enqueued.
//!
//! # Safety
//!
//! This crate contains unavoidable `unsafe` blocks in the ring buffer for
//! atomic, lock-free access to shared memory from ISR context. These are
//! isolated with documented safety invariants.
//!
//! Exception granted per workspace policy: this is the ring buffer boundary
//! crate requiring low-level atomic operations.
//!
//! # Example
//!
//! ```rust
//! use tpt_e_chronos::ring_buf::RingBuf;
//!
//! let buf = RingBuf::<u32, 8>::new(0);
//! assert!(buf.push(42).is_ok());
//! assert_eq!(buf.pop(), Some(42));
//! assert!(buf.is_empty());
//! ```

pub mod dma_handoff;
pub mod ring_buf;

#[cfg(feature = "mock")]
pub mod mock;
