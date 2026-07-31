#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-cipher`
//!
//! Mathematically verified, constant-time wrappers for ESP32 hardware crypto
//! accelerators (AES, SHA-256, ECC).
//!
//! The API is trait-based, abstracting over the `esp-hal` crypto peripherals.
//!
//! ## Constant-time status
//!
//! - **AES** ([`aes`]) and **SHA-256** ([`sha`]): fixed-operation-count,
//!   no data-dependent branches on secret material.
//! - **ECC** ([`ecc`]): scalar multiplication's point arithmetic and
//!   accumulator selection are branch-free on secret data (complete
//!   addition/doubling formulas plus a bitmask select), but the underlying
//!   big-integer field arithmetic (modular reduction, inversion, comparison)
//!   is not yet constant-time — see [`ecc`]'s module docs for the precise
//!   boundary. Do not use ECC signing where timing side channels on the
//!   private key or nonce are in the threat model until that layer is
//!   addressed.

pub mod aes;
pub mod sha;
pub mod ecc;
pub mod traits;

mod sha256_core;

#[cfg(feature = "mock")]
pub mod mock;
