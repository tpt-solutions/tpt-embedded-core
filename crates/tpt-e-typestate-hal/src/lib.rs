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

// Real, hardware-verified DMA-driven AES typestate channel. Only compiled
// for the chips esp-hal actually exposes `aes::dma::AesDma` on — see
// `aes_dma`'s module docs for why this exists separately from the still-stub
// generic `DmaChannel<State, EspHalBackend>`.
#[cfg(any(feature = "esp32c3", feature = "esp32c6", feature = "esp32s3"))]
pub mod aes_dma;

#[cfg(feature = "mock")]
pub mod mock;
