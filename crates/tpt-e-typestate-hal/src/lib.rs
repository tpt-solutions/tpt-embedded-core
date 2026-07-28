#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-typestate-hal`
//!
//! Compile-time safe DMA and ISR abstractions for the ESP32 family.
//!
//! Uses the typestate pattern to enforce correct peripheral state transitions
//! at compile time: `Idle → Configured → Transferring → Complete`.
//!
//! # Safety
//!
//! This crate contains unavoidable `unsafe` blocks at the boundary with
//! hardware (ISR registration, DMA descriptor manipulation). These are
//! isolated in dedicated modules with documented safety invariants.
//!
//! Exception granted per workspace policy: this is the foundational HAL
//! boundary crate.

pub mod backend;
pub mod dma;
pub mod isr;
pub mod state;

#[cfg(feature = "mock")]
pub mod mock;
