# `aes-dma-smoke`

Real-hardware DMA smoke test. Unlike `hil-hello` (which only proves the
flash/monitor pipeline works), this drives an actual peripheral — the
ESP32-C3's hardware AES engine — through a real DMA channel
(`esp_hal::aes::dma::AesDma`), then compares the result against the
FIPS-197 Appendix B known-answer vector (the same key/plaintext/ciphertext
already used and verified in `crates/tpt-e-cipher/src/aes.rs`'s
`aes128_nist_vector` test, against this repo's own software AES).

No external wiring needed — AES-DMA moves data between RAM and the AES
engine entirely inside the chip, unlike e.g. SPI-DMA loopback (which needs
a physical MOSI–MISO jumper).

## Result (2026-07-29, real ESP32-C3 rev v0.4)

```
AES-DMA PASS: hardware ciphertext matches FIPS-197 vector
```

Confirmed on the first flash. This proves the underlying esp-hal DMA+AES
API this repo would eventually wrap in `tpt-e-typestate-hal`/`tpt-e-cipher`
actually works correctly on real silicon — a concrete data point for (not a
replacement of) the still-open `EspHalBackend` real-DMA-implementation gap
in the root `todo.md`.

## Flashing

```bash
cd firmware
cargo run --release -p aes-dma-smoke   # builds, flashes, and monitors via espflash
```

See `../hil-hello/README.md` for the `espflash` version gotcha (must be
`3.x`, not `4.5.0`) that applies here too.
