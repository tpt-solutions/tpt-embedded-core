# Getting Started

## Prerequisites

- Rust toolchain (stable for host testing, stable+nightly for embedded)
- `mdbook` for building documentation: `cargo install mdbook`

## Quick Start (Host Testing)

Every crate supports host-side testing with the `mock` feature:

```bash
# Run all tests
cargo test --workspace --features mock

# Run property-based tests
cargo test --features mock --release
```

## Using a Single Crate

Add to your `Cargo.toml`:

```toml
[dependencies]
tpt-e-chronos = { version = "0.1", features = ["mock"] }
```

Then use it:

```rust
use tpt_e_chronos::ring_buf::RingBuf;

fn main() {
    let buf = RingBuf::<u32, 8>::new(0);
    buf.push(42).unwrap();
    assert_eq!(buf.pop(), Some(42));
}
```

## Building Documentation

```bash
cd docs
mdbook build
mdbook serve  # localhost:3000
```

## Next Steps

- [Cross-Crate Wiring](./cross-crate-wiring.md) — connect crates together
- [Philosophy](./philosophy.md) — understand the design principles
- [Contributing](./contributing.md) — how to contribute
