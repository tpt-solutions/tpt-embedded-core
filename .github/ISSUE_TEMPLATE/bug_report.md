---
name: Bug report
about: Something in tpt-embedded-core doesn't work as documented or verified
title: ""
labels: bug
---

## Which crate(s)

<!-- e.g. tpt-e-typestate-hal, tpt-e-chronos, tpt-e-cipher, tpt-e-slumber, tpt-e-swarm-sync -->

## What happened

<!-- What you did, what you expected, what actually happened. -->

## Reproduction

<!--
A minimal `cargo test`/`cargo build`/`cargo run` command (with features and
target, if relevant) that shows the problem. If it's a hardware-only bug,
say which chip (esp32/esp32s3/esp32c3/esp32c6) and board revision.
-->

## Which guarantee broke, if any

<!--
This project's review checklist (CONTRIBUTING.md) claims specific
guarantees per crate — typestate transitions that should be compile
errors, proptest/Kani-proven invariants, WCET bounds, constant-time
execution. If this bug means one of those guarantees doesn't actually
hold (e.g. an invalid transition compiled, a Kani harness didn't cover the
failing case, a "mock"-only code path breaks on a real chip target),
naming which one helps prioritize — those are treated as more severe than
a plain logic bug.
-->

## Environment

- Rust version: `rustc --version`
- Target (if embedded): e.g. `riscv32imc-unknown-none-elf`
- OS (if host-side): e.g. Windows/Linux/macOS
