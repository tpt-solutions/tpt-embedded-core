# Formal Verification Tooling

## Kani (in use)

`cargo kani` is the workspace's primary formal verification tool — bounded
model checking over symbolic inputs, run in CI via `kani.yml`. It proves
absence of panics/overflows/UB and specific safety-invariant harnesses (ring
buffer bounds, typestate transitions, mesh-state divergence-freedom) across
*all* reachable executions up to the tool's unwinding bounds. See
[CI Pipeline](./ci-pipeline.md) for how it's wired in and each crate's page
for what's actually proven vs. still open.

## Creusot (evaluated 2026-07-29, not adopted)

`spec.txt` §7 mentions Creusot alongside Kani as a candidate formal
verification tool but doesn't detail it further, and `todo.md` tracks
evaluating it as a stretch goal. This page records that evaluation.

### What it is

[Creusot](https://creusot.rs/) is a *deductive* verifier: instead of bounded
model checking, it translates Rust to an intermediate language (Coma) and
asks off-the-shelf SMT solvers (Alt-Ergo, Z3, CVC4/CVC5) via the Why3
platform to prove that code satisfies explicit `#[requires]`/`#[ensures]`
contracts, written in a specification language called Pearlite. Where Kani
answers "does any input up to this bound trigger a panic/assertion
failure?", Creusot answers "does this function *always* satisfy this
functional contract?" — a stronger, unbounded claim, but one that requires
writing (and trusting) the contract itself, and one that can fail to
discharge (timeout) rather than cleanly succeed or find a counterexample.

### Toolchain fit for this workspace

- **Requires Linux or macOS** (`x86_64-linux`, `aarch64-darwin` per the
  install guide) — no Windows support. Not a blocker: CI already runs
  `kani.yml` on `ubuntu-latest`, and Creusot would run there too, just not
  on a Windows dev machine directly (matches this repo's existing situation
  with Kani, which also doesn't build natively on Windows — see the
  swarm-sync Kani proof notes in `todo.md`).
- **Pinned nightly toolchain + an OCaml/Opam solver stack** (Why3, why3find,
  Alt-Ergo, Z3, CVC4/CVC5) installed separately from the crate's own stable
  toolchain. This is a heavier one-time CI setup than Kani's single
  `cargo-kani` install.
- **`no_std` support**: Creusot's `creusot_std` can build against `core`
  only, without requiring `alloc`. This workspace uses neither `std` nor
  `alloc` anywhere (verified: no crate references `extern crate alloc` or
  `alloc::`), so it fits the no-`alloc` case Creusot explicitly supports —
  no architectural mismatch there.
- **Concurrency support is new and narrow** (as of Creusot 0.9.0, January
  2026): a first `AtomicI32` ghost-aware wrapper plus an `AtomicInvariant`
  concept, aimed at simple examples like `parallel_add`. `tpt-e-chronos`'s
  `RingBuf` push/pop critical section (an `AtomicBool` spinlock, not
  `AtomicI32`) is exactly the kind of code this line of work targets, but
  it's too early-stage to assume it would handle a real spinlock-guarded
  ring buffer today.

### Where it could add value beyond Kani

Kani's ring-buffer/typestate harnesses prove *panic-freedom* and specific
hand-written invariants (e.g. "push then pop returns what was pushed" as a
harness assertion) up to bounded unwinding. Creusot contracts would let
those same properties be stated once as part of the function signature
(`#[ensures(...)]` on `RingBuf::push`/`pop`) and proven for *all* capacities
and interleavings the type permits, not just the ones a harness happens to
exercise. The clearest candidates in this workspace are the const-generic
`RingBuf` invariants and the typestate transition guarantees in
`tpt-e-typestate-hal` — both are exactly the "prove this holds for every
valid input, not just sampled ones" shape deductive verification is good at.

### Recommendation

Stay a stretch goal, not adopted now. Reasons:

1. Kani already covers this workspace's actual safety claims (panic/UB
   freedom, the specific invariants documented per-crate) reasonably well.
2. Adding a second, heavier proof toolchain (new CI job, Opam/solver
   install, nightly pin, a new contract language to learn and maintain)
   is a real ongoing cost, not a one-time add.
3. Creusot's concurrency support — relevant to `tpt-e-chronos`'s spinlock
   and `tpt-e-swarm-sync`'s state machine — is too new to bet on.

Revisit this if either becomes true: (a) the project wants unbounded
functional-correctness contracts (not just panic-freedom) as a stated goal
rather than a nice-to-have, or (b) Creusot's atomic/concurrency support
matures enough to plausibly model `RingBuf`'s critical section.
