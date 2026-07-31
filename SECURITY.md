# Security Policy

`tpt-embedded-core` ships real cryptographic primitives — AES-128, SHA-256,
and P-256 ECDSA in `tpt-e-cipher` — with explicit, documented constant-time
guarantees. Security reports against this project are not limited to memory
safety and undefined behavior; **timing/side-channel behavior that
contradicts the documented constant-time boundary is explicitly in scope.**

## Reporting a vulnerability

Please report suspected vulnerabilities privately, not via a public GitHub
issue:

- **Preferred:** use [GitHub's private vulnerability
  reporting](https://github.com/tpt-solutions/tpt-embedded-core/security/advisories/new)
  ("Security" tab → "Report a vulnerability") on this repository. This opens
  a private advisory visible only to maintainers until a fix is ready.

Please include:

- The affected crate(s) and version/commit.
- A minimal reproduction or a description of the observable behavior
  (timing measurements, a failing invariant, a crash input, etc.).
- Which of this project's specific guarantees is violated — e.g. "AES-128
  encryption is not constant-time for X," "a typestate transition allows Y
  at runtime," "a Kani-proven invariant in Z is actually reachable" — since
  the project's threat model is defined by its documented proofs and
  disclosed gaps (below), not a generic checklist.

## Known, already-disclosed gaps (not new reports)

The following are known limitations, already documented and tracked in
`todo.md` and the affected crates' `CHANGELOG.md` "Known limitations"
sections — no need to report these as new findings, though evidence they're
worse than documented (e.g. an actual measured timing leak, not just the
theoretical branch-on-secret-data shape) is still useful:

- `tpt-e-cipher`'s P-256 ECDSA: point arithmetic (`point_add`/`point_double`
  via `point_mul`) is branch-free on secret data, but the underlying field
  arithmetic (`fe_add`/`fe_sub`/`fe_inv`, `u256_cmp`) still uses
  value-dependent branches — see `crates/tpt-e-cipher/src/ecc.rs`'s module
  doc and `docs/src/formal-verification.md`.
- `tpt-e-typestate-hal`'s `EspHalBackend` (the generic DMA backend) is an
  intentional no-op stub — see its module doc for why.
- `tpt-e-slumber`'s proof tokens are not yet wired to real
  precondition-checks from other crates — they currently only prevent
  forging tokens from outside the crate, not real DMA/RTC/buffer state.
- No library crate has been validated against real ESP32 hardware yet — see
  the root `README.md`'s "Status" section for exactly what has and hasn't
  been hardware-tested.

## Supported versions

This project is pre-1.0 (`0.1.0`, workspace-synced across all five crates)
and not yet published to crates.io. Until a 1.0 release, only the `master`
branch is supported — please report against the latest commit.
