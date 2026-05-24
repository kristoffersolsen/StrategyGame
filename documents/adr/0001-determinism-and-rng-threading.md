# ADR-0001: Determinism and RNG Threading

**Status:** Accepted  
**Date:** 2026-05-24  
**Deciders:** Kristoffer

## Context

The project requires byte-identical world generation across machines and
across save/load round-trips (design doc §0.4). This means random number
generation must be:

1. **Seeded** — deterministic for a given master seed.
2. **Stage-independent** — tweaking the parameters of generation stage N
   must not shift the output of any other stage M, otherwise iterative
   tuning is impossible: every parameter change cascades through the
   entire pipeline.
3. **Order-independent** — the sub-seed for stage N must not depend on
   how many random numbers stage N-1 happened to consume, since that
   count is an implementation detail subject to change.
4. **Stable across Rust releases** — the standard library's `DefaultHasher`
   is explicitly documented as unstable across toolchain versions. Any
   strategy relying on it would silently break reference hashes on a
   toolchain bump.

The project will need deterministic RNG in two distinct contexts:

- **World generation** (`worldgen`) — one-shot, runs at game start, sub-RNGs
  keyed by pipeline stage. No save/load requirement.
- **Runtime game loop** (`game`) — runs every turn for the lifetime of a
  game, sub-RNGs keyed by Bevy system and player. Must survive
  save/load round-trips with the RNG state serialized into the save file.

These contexts share the same underlying need (deterministic sub-seed
derivation from a master seed) but differ enough in their higher-level
API that a single shared API would serve neither well.

## Decision

### Algorithm

Use **`SplitMix64`** as the sub-seed mixer. Given a master seed `u64` and a
domain-specific salt `u64`, the sub-seed is:

```rust
let m = splitmix64(master);
let s = splitmix64(salt);
splitmix64(m ^ s.wrapping_add(0x9E37_79B9_7F4A_7C15))
```

The two pre-mix passes before the XOR prevent the trivial collision that
`splitmix64(master ^ salt)` has when two distinct `(master, salt)` pairs
have the same XOR value. The addend (`0x9E37...`) is the golden-ratio
constant from the SplitMix64 reference and breaks the symmetry of the
raw XOR further.

`SplitMix64` was chosen for sub-seed derivation because it was designed
for exactly this use case (Java's `SplittableRandom` uses the same
algorithm for splitting independent sub-streams), is five lines of
arithmetic with no dependencies, and is not cryptographic (we do not need
cryptographic guarantees inside a trust boundary).

### Base RNG type

Use **`rand_chacha::ChaCha8Rng`** for all random streams. ChaCha8's output
for a given seed is documented as stable by the `rand_chacha` crate.
`ChaCha8` (8 rounds) rather than `ChaCha20` (20 rounds) is sufficient for
non-cryptographic game use and is faster.

### Crate structure

The shared primitive — the mixer, the `derive_seed` function, and a
`DetRng` newtype around `ChaCha8Rng` — lives in a new zero-dependency
crate **`det_rng`**. Both `worldgen` and `game` depend on it.

The higher-level threading APIs live in their respective consumers:

- `worldgen::rng` exposes `MasterRng`, `StageRng`, and `StageId`. A
  `MasterRng` is constructed from the master seed. Calling
  `master.fork(StageId::Climate)` produces a fresh `StageRng` whose seed
  is `derive_seed(master_seed, StageId::Climate as u64)`. Each `StageRng`
  implements `RngCore` and is passed directly to the stage function.
- The future `game` crate will expose `GameRng` and a `SystemId` type,
  using the same `derive_seed` primitive, with save/load serialization
  of the RNG state added at that time.

### `StageId` salt values

`StageId` variants are assigned explicit `u64` salt constants rather than
derived from enum discriminants. This prevents a reordering of the enum
from silently shifting every sub-seed and invalidating all reference
hashes.

### Hard rules enforced by lint and review

- **No `rand::thread_rng` / `rand::rng`** anywhere in the workspace.
  The clippy `disallowed_methods` lint is configured to forbid both.
  All randomness must flow from a `DetRng` seeded by `derive_seed`.
- **No `HashMap` iteration whose order affects generation output.** Use
  `BTreeMap`, or collect and sort before iterating. Clippy cannot enforce
  this; it is a code review gate.
- **No parallel reductions in inner generation loops** unless an explicit
  deterministic reduction is used. Same enforcement model as above.

## Consequences

**Positive:**

- Tuning any single generation stage does not shift the output of any
  other stage, making iterative parameter tuning tractable.
- The same master seed produces byte-identical `WorldData` on every
  machine and across every rebuild, satisfying the multiplayer-safe
  requirement from §0.4.
- The shared primitive (`det_rng`) has zero project dependencies and
  compiles in milliseconds.
- Explicit `StageId` salt constants mean enum reordering is safe.

**Negative:**

- Two RNG idioms in the codebase once the runtime system is added
  (`worldgen::MasterRng` and the future `game::GameRng`). This is
  acceptable because the contexts are genuinely different.
- `SplitMix64` is not cryptographic. If a future feature requires
  adversarially-secure randomness (e.g., hidden information in an
  asynchronous multiplayer game), this strategy does not cover it and a
  separate mechanism will be needed.

## Alternatives considered

**`rand_seeder::Seeder`** — a purpose-built crate for deriving seeds from
arbitrary hashable values. Rejected in favour of an inline `SplitMix64`
implementation because the hand-rolled form is five lines, has no
additional transitive dependencies, and makes the algorithm explicit at
the call site rather than hiding it behind a library boundary.

**`std::hash::SipHasher`** — available in `std`, but deprecated since
Rust 1.13.0. The replacement, `DefaultHasher`, is explicitly documented
as unstable across releases, which would break reference hashes on a
toolchain bump. Rejected.

**`blake3` (reused from the determinism scaffold)** — cryptographic hash,
significantly more code than `SplitMix64`, and overkill for a trust
boundary where adversarial inputs do not apply. Rejected for sub-seed
derivation; `blake3` remains the right choice for the reference-hash
determinism tests in the scaffold (ADR to follow).

**Single shared crate for all RNG APIs** — putting `MasterRng`, `StageRng`,
`GameRng`, and `SystemId` all in `det_rng`. Rejected because stage-keyed
and system-keyed APIs differ in their save/load requirements, their key
types, and their lifecycle. A single API would require enough generic
parameters or enums to serve both that it would be harder to read and
use than two purpose-built APIs sharing a common primitive.

## References

- Design doc §0.4 — multiplayer-safe-by-default pillar
- Design doc §1.10 — climate advection loop; deterministic iteration order
- `crates/det_rng/src/lib.rs` — implementation
- Vigna, S. — splitmix64 reference implementation: https://prng.di.unimi.it/splitmix64.c
- `rand_chacha` stability guarantee: https://docs.rs/rand_chacha