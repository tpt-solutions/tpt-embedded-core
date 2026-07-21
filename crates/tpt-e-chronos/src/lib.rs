#![no_std]
#![allow(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-chronos`
//!
//! A WCET-bounded, zero-allocation ring buffer for time-series telemetry data.
//!
//! The ring buffer uses a const-generic capacity and supports ISR-safe push (via
//! critical sections) and main-loop pop operations. It provides a zero-copy
//! handoff path to DMA via the `tpt-e-typestate-hal` safe handles.
//!
//! # Safety
//!
//! This crate contains unavoidable `unsafe` blocks in the ring buffer for
//! atomic, lock-free access to shared memory from ISR context. These are
//! isolated with documented safety invariants.
//!
//! Exception granted per workspace policy: this is the ring buffer boundary
//! crate requiring low-level atomic operations.

pub mod dma_handoff;
pub mod ring_buf;

#[cfg(feature = "mock")]
pub mod mock;
