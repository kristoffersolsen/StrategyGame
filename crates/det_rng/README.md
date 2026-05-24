# det-rng

Deterministic-RNG primitives shared across the strategy game project.

Provides [`DetRng`] (a newtype wrapper around `rand_chacha::ChaCha8Rng`
with a documented stability guarantee), [`derive_seed`] (combines a
master seed with a salt to produce an independent sub-seed), and
[`splitmix64`] (the underlying bit mixer).

This crate provides only the shared primitive. The higher-level APIs
that drive sub-RNG threading — `worldgen::MasterRng` /
`worldgen::StageRng` for generation, the future `game::GameRng` for
runtime — are built on top in their respective crates, because
their needs (stage-keyed vs system-keyed, one-shot vs save/load-aware)
differ enough that a single API would serve neither well.

## Starting point

See [`DetRng`] and [`derive_seed`] in `src/lib.rs`.

## Further reading

- `docs/design/overview.md` §0.4 — multiplayer-safe-by-default pillar
- `docs/adr/0009-determinism-and-rng-threading.md` — the workspace
  determinism strategy (forthcoming)