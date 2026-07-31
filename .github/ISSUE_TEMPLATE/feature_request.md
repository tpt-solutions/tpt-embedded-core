---
name: Feature request
about: Propose new functionality or a change to an existing crate's API
title: ""
labels: enhancement
---

## What problem does this solve

<!-- The use case or gap this addresses, not just the feature itself. -->

## Which crate(s)

<!-- e.g. tpt-e-typestate-hal, tpt-e-chronos, tpt-e-cipher, tpt-e-slumber, tpt-e-swarm-sync, or a new crate -->

## Proposed approach

<!--
Sketch the API if you have one in mind. This project's design philosophy
(see docs/src/philosophy.md and CONTRIBUTING.md's review checklist) leans
on:
- Typestate over runtime checks — invalid states should be unrepresentable
  at compile time, not caught by a runtime `Result`/panic.
- `#![deny(unsafe_code)]` with any unsafe isolated to a minimal, documented
  boundary.
- Proptest/Kani coverage for new invariants, not just unit tests.
- Deterministic, bounded execution time (WCET) for public API paths.
Proposals that fit this shape are easier to review and land.
-->

## Alternatives considered

<!-- Other approaches you thought about, and why this one's preferred. -->
