#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-cipher`
//!
//! Mathematically verified, constant-time wrappers for ESP32 hardware crypto
//! accelerators (AES, SHA-256, ECC).
//!
//! The API is trait-based, abstracting over the `esp-hal` crypto peripherals.
//! All public operations guarantee constant-time execution paths regardless of
//! input data or secret key material.

pub mod aes;
pub mod sha;
pub mod ecc;
pub mod traits;

mod sha256_core;

#[cfg(feature = "mock")]
pub mod mock;
