# Contributing

See the full [CONTRIBUTING.md](https://github.com/tpt-solutions/tpt-embedded-core/blob/master/CONTRIBUTING.md)
for the TPT Standard review checklist.

## Quick Reference

1. All new code must have `#![deny(unsafe_code)]` at the crate root
2. Unsafe exceptions require a documented justification in the module doc
3. Every public API needs:
   - `#[warn(missing_docs)]` coverage
   - At least one `proptest` property test
   - A Kani proof for critical invariants
4. Typestate patterns preferred over runtime checks
5. No heap allocation in hot paths
