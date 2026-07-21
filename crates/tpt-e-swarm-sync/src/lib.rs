#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations)]

//! `tpt-e-swarm-sync`
//!
//! Deterministic, state-consistent coordination for ESP32 swarms over ESP-NOW
//! or 802.15.4.
//!
//! A formally verified state machine (derived from a TLA+ specification)
//! handles message sequencing, acknowledgments, and partition recovery.

pub mod mesh;
pub mod state_machine;

#[cfg(feature = "mock")]
pub mod mock;
