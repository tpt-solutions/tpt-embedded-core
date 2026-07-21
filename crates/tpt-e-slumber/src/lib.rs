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
//!
//! # Typestate Guarantee
//!
//! Deep sleep can only be entered when **all** required proof tokens are provided:
//!
//! - `DmaParkedToken`: Proves all DMA channels are parked.
//! - `RtcIsolatedToken`: Proves RTC memory is isolated.
//! - `BuffersFlushedToken`: Proves all transmit buffers are flushed.
//!
//! If any token is missing, the code fails to compile — not at runtime.
//!
//! Tokens can only be minted by the crate itself (or, under the `mock`
//! feature, via `Token::mock()` for host-side testing) — there is no public
//! `Token::new()` that arbitrary calling code can use to fabricate a proof
//! without a real subsystem backing it.
//!
//! # Example
//!
//! This example requires the `mock` feature (`cargo test --features mock`),
//! since token construction outside the crate is only available for
//! host-side testing.
//!
//! ```rust
//! use tpt_e_slumber::sleep::SleepController;
//! use tpt_e_slumber::tokens::{DmaParkedToken, RtcIsolatedToken, BuffersFlushedToken};
//!
//! let controller = SleepController::new();
//! let dma_token = DmaParkedToken::mock();
//! let rtc_token = RtcIsolatedToken::mock();
//! let buffers_token = BuffersFlushedToken::mock();
//!
//! // This would enter deep sleep (returns `!`):
//! // controller.enter_deep_sleep(dma_token, rtc_token, buffers_token);
//! ```
//!
//! # Compile-time Safety
//!
//! The following fails to compile because no tokens are provided:
//!
//! ```compile_fail
//! use tpt_e_slumber::sleep::SleepController;
//!
//! let controller = SleepController::new();
//! controller.enter_deep_sleep(); // Error: missing tokens
//! ```

pub mod sleep;
pub mod tokens;

#[cfg(feature = "mock")]
pub mod mock;
