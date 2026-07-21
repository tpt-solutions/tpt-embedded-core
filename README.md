# `tpt-embedded-core`

A proof-native, formally verified `no_std` foundation for ESP32 ecosystems.

## Philosophy

- **Typestate over Runtime Checks** — Invalid states are unrepresentable at compile time.
- **`#![deny(unsafe_code)]`** — Unsafe code is isolated, documented, and formally verified.
- **Property-Based Testing** — Core invariants proven by fuzzing and model checking.
- **WCET-Bounded** — Every public function has a deterministic Worst-Case Execution Time.

## Architecture

```text
tpt-embedded-core/
├── tpt-e-typestate-hal/     # [Foundation] Compile-time safe DMA & ISR abstractions
├── tpt-e-chronos/           # [Data] WCET-bounded telemetry ring buffer
├── tpt-e-cipher/            # [Security] Formally verified HW crypto wrappers
├── tpt-e-slumber/           # [Power] Compile-time verified sleep transitions
└── tpt-e-swarm-sync/        # [Network] Verified mesh coordination state machine
```

### Crate Dependencies

| Crate | Depends On | Status |
|---|---|---|
| `tpt-e-typestate-hal` | — (foundation) | Phase 1 |
| `tpt-e-chronos` | `tpt-e-typestate-hal` (DMA handles) | Phase 1 |
| `tpt-e-slumber` | `tpt-e-typestate-hal` (state tokens) | Phase 2 |
| `tpt-e-cipher` | `tpt-e-typestate-hal` (DMA handles) | Phase 3 |
| `tpt-e-swarm-sync` | `tpt-e-chronos` (message queuing) | Phase 4 |

## Getting Started

```bash
cargo test --features mock --workspace
```

## Versioning & Publishing

All crates in this workspace share a single workspace version (`0.1.0`). Each
crate is independently publishable to crates.io but follows a synchronized
versioning strategy during early development (Phase 0–4). Post-stabilization,
breaking changes to `tpt-e-typestate-hal` will trigger a minor version bump
for all dependent crates.

## License

Dual-licensed under MIT OR Apache-2.0.
