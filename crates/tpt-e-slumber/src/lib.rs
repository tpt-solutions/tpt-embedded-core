#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-slumber`
//!
//! Compile-time verified power management and sleep transitions for the ESP32 family.
//!
//! Uses a proof-token API — subsystems issue tokens (e.g., `DmaParkedToken`,
//! `RtcIsolatedToken`) only when they can prove their precondition is satisfied.
//! Sleep transitions are gated on collecting the required tokens at compile time.

pub mod sleep;
pub mod tokens;

#[cfg(feature = "mock")]
pub mod mock;
