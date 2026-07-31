//! A real, hardware-verified DMA-driven typestate channel — `esp-hal`'s
//! AES-DMA peripheral.
//!
//! # Why this exists instead of a generic `DmaChannel<State, EspHalBackend>`
//!
//! `backend::EspHalBackend` (the intended real-hardware backend for the
//! generic `DmaChannel<State, B>` in [`crate::dma`]) is still a stub. That
//! generic API assumes a bare "move this memory buffer via DMA, unmodified"
//! operation — but on ESP32/C3/S3 (this crate's target chips), no such
//! peripheral exists: `esp_hal::dma::m2m::Mem2Mem` (the closest match) is
//! only available on esp32c2/esp32c6/esp32h2, not esp32c3, esp32s3, or the
//! original esp32. The one DMA-driven peripheral path confirmed to actually
//! work on real hardware (see `firmware/aes-dma-smoke` in this repo, flashed
//! and verified against the FIPS-197 Appendix B vector on a real ESP32-C3)
//! is `esp_hal::aes::dma::AesDma` — which necessarily *transforms* data
//! (encrypts/decrypts) rather than moving it unmodified, so it cannot
//! honestly implement the generic bare-buffer contract either.
//!
//! Rather than block on that unresolved generic-vs-peripheral-specific
//! design question, this module provides a second, concrete, peripheral-
//! specific typestate channel that mirrors the same `Idle → Configured →
//! Transferring → Complete` shape (reusing the same marker types from
//! [`crate::state`]) but is honest about what it wraps: AES specifically,
//! not an arbitrary memory-to-memory move.
//!
//! # Why `start()` performs the entire transfer synchronously
//!
//! `esp_hal::aes::dma::AesDma`'s DMA channel is fixed to `Blocking` mode —
//! there is no non-blocking/async variant in this esp-hal version. The
//! handle `AesDma::process()` returns
//! (`DmaTransferRxTx<'t, AesDma>`) also borrows the `AesDma` instance for
//! `'t`, which makes storing an in-flight transfer as a field of a
//! separately-movable `Transferring` value a self-referential-struct
//! problem — solvable, but only with additional `unsafe` lifetime
//! extension. Since the underlying driver is blocking anyway (there is no
//! real "do other work while this runs" window to preserve), `start()`
//! calls `process()` and `wait()` back-to-back and stores the outcome. The
//! four-state shape is kept for API consistency with [`crate::dma`] (and
//! because it still gives real compile-time transition safety — you cannot
//! call `wait()` before `start()`), even though, for this specific
//! peripheral, the hardware operation is already finished by the time
//! `Transferring` is observed. No new `unsafe` is introduced by this module.
//!
//! # Chip availability
//!
//! Only compiled when the `esp32c3`, `esp32c6`, or `esp32s3` feature is
//! selected (mirroring `esp-hal`'s own `#[cfg(any(esp32c3, esp32c6, esp32h2,
//! esp32s3))]` gate on `aes::dma`). The plain `esp32` feature does not get
//! this module — esp-hal has no `AesDma` for that chip — which is a real
//! hardware gap, not an oversight.

use core::marker::PhantomData;

#[cfg(feature = "defmt")]
use defmt::trace;

use esp_hal::aes::{
    dma::{AesDma, CipherMode},
    Key, Mode,
};
use esp_hal::dma::DmaError;

use crate::state::{Complete, Configured, Idle, Transferring};

/// AES mode + cipher mode, captured at [`AesDmaChannel::configure`] time and
/// used when the transfer actually runs in
/// [`AesDmaChannel::start`][configured-start].
///
/// [configured-start]: AesDmaChannel::start
// No `derive` here: `esp_hal::aes::Mode`/`CipherMode` implement neither
// `Debug`, `Clone`, nor `Copy`.
struct AesConfig {
    mode: Mode,
    cipher_mode: CipherMode,
}

/// A DMA-driven AES channel parameterised by its current typestate.
///
/// Wraps a real `esp_hal::aes::dma::AesDma` (obtained via
/// `Aes::new(peripherals.AES).with_dma(channel, rx_descriptors,
/// tx_descriptors)`) and drives it through the same `Idle → Configured →
/// Transferring → Complete` shape used elsewhere in this crate. See the
/// module docs for why this exists as a separate, peripheral-specific type
/// rather than an implementation of the generic [`crate::dma::DmaChannel`].
pub struct AesDmaChannel<'d, State> {
    _state: PhantomData<State>,
    aes_dma: AesDma<'d>,
    config: Option<AesConfig>,
    result: Option<Result<(), DmaError>>,
}

impl<'d> core::fmt::Debug for AesDmaChannel<'d, Idle> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AesDmaChannel<Idle>").finish_non_exhaustive()
    }
}

impl<'d> core::fmt::Debug for AesDmaChannel<'d, Configured> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Can't print `config` itself: `esp_hal::aes::{Mode, CipherMode}`
        // implement neither `Debug` nor `Copy`/`Clone`.
        f.debug_struct("AesDmaChannel<Configured>")
            .finish_non_exhaustive()
    }
}

impl<'d> core::fmt::Debug for AesDmaChannel<'d, Transferring> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AesDmaChannel<Transferring>")
            .finish_non_exhaustive()
    }
}

impl<'d> core::fmt::Debug for AesDmaChannel<'d, Complete> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AesDmaChannel<Complete>")
            .field("result", &self.result)
            .finish_non_exhaustive()
    }
}

impl<'d> AesDmaChannel<'d, Idle> {
    /// Wrap an already-constructed `esp_hal::aes::dma::AesDma` instance.
    ///
    /// Typical construction (see `firmware/aes-dma-smoke` for a full
    /// working example):
    ///
    /// ```ignore
    /// let dma = esp_hal::dma::Dma::new(peripherals.DMA);
    /// let dma_channel = dma.channel0.configure(false, DmaPriority::Priority0);
    /// let (rx_descriptors, tx_descriptors) = esp_hal::dma_descriptors!(16, 16);
    /// let aes = esp_hal::aes::Aes::new(peripherals.AES);
    /// let aes_dma = aes.with_dma(dma_channel, rx_descriptors, tx_descriptors);
    /// let channel = AesDmaChannel::new(aes_dma);
    /// ```
    pub fn new(aes_dma: AesDma<'d>) -> Self {
        Self {
            _state: PhantomData,
            aes_dma,
            config: None,
            result: None,
        }
    }

    /// Configure the AES mode (encrypt/decrypt, key width) and cipher mode
    /// (ECB, CBC, ...) for the upcoming transfer.
    pub fn configure(self, mode: Mode, cipher_mode: CipherMode) -> AesDmaChannel<'d, Configured> {
        AesDmaChannel {
            _state: PhantomData,
            aes_dma: self.aes_dma,
            config: Some(AesConfig { mode, cipher_mode }),
            result: None,
        }
    }
}

impl<'d> AesDmaChannel<'d, Configured> {
    /// Start (and, per the module docs, fully run) the AES-DMA transfer.
    ///
    /// `key` accepts anything `esp_hal::aes::Key` can be built from — in
    /// practice `[u8; 16]`, `[u8; 24]`, or `[u8; 32]` depending on the
    /// configured [`Mode`].
    pub fn start(
        mut self,
        key: impl Into<Key>,
        plaintext: &[u8],
        mut ciphertext: &mut [u8],
    ) -> AesDmaChannel<'d, Transferring> {
        #[cfg(feature = "defmt")]
        trace!("AesDmaChannel::start: key_len={:?}, pt_len={:?}, ct_len={:?}", key.into().as_bytes().len(), plaintext.len(), ciphertext.len());

        let AesConfig { mode, cipher_mode } =
            self.config.take().expect("Configured implies config set");

        let result = self
            .aes_dma
            .process(&plaintext, &mut ciphertext, mode, cipher_mode, key)
            .and_then(|transfer| transfer.wait());

        #[cfg(feature = "defmt")]
        trace!("AesDmaChannel::start done: result={:?}", result.is_ok());

        AesDmaChannel {
            _state: PhantomData,
            aes_dma: self.aes_dma,
            config: self.config,
            result: Some(result),
        }
    }
}

impl<'d> AesDmaChannel<'d, Transferring> {
    /// Wait for the transfer to complete, returning the result and the
    /// channel in the `Complete` state.
    ///
    /// Per the module docs, the transfer has already run to completion by
    /// the time this is reachable (the underlying driver is blocking) — this
    /// method exists for API-shape consistency with [`crate::dma`], and so
    /// consumers can write code generic over "when is the result available"
    /// without caring that this particular peripheral has no real overlap
    /// window.
    pub fn wait(self) -> AesDmaChannel<'d, Complete> {
        AesDmaChannel {
            _state: PhantomData,
            aes_dma: self.aes_dma,
            config: self.config,
            result: self.result,
        }
    }
}

impl<'d> AesDmaChannel<'d, Complete> {
    /// The result of the completed transfer.
    pub fn result(&self) -> Result<(), DmaError> {
        #[cfg(feature = "defmt")]
        trace!("AesDmaChannel::result: {:?}", self.result.is_ok());
        // SAFETY-free: no `unsafe` here. `result` is always `Some` because
        // the only way to reach `Complete` is via `Transferring::wait`,
        // which always carries forward a `Some` set by `Configured::start`.
        self.result.expect("Complete implies result set")
    }

    /// Consume this channel, releasing the underlying `AesDma` instance and
    /// resetting back to `Idle` so it can be reused for another transfer.
    pub fn release(self) -> AesDmaChannel<'d, Idle> {
        AesDmaChannel {
            _state: PhantomData,
            aes_dma: self.aes_dma,
            config: None,
            result: None,
        }
    }
}
