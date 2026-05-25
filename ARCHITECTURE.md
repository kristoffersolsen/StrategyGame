# Architecture

This document describes the high-level structure of the codebase:
what the crates are, what they own, and how they depend on each other.
It is updated at each stage gate as new structure is added.

For *why* the architecture is this way, see `docs/design/overview.md`.
For *when* decisions were made, see `documents/adr/`.

**Current stage:** Stage 3.1 complete. Stage 3.2 (climate) next.

---

## Workspace layout
crates/ \
coords/     — wrap-aware hexagonal coordinate math (no dependencies) \
det_rng/    — deterministic RNG primitives (no project dependencies) \
worldgen/   — procedural world generation pipeline (depends on coords, det_rng) \
game/       — Bevy application and runtime (depends on coords, worldgen, det_rng) \
tools/ \
mapdump/    — CLI: dump a single seed's generation output to PNG \
maptest/    — CLI: multi-seed runs and parameter-range sweeps (rayon parallel)

## Crate responsibilities

### `coords`

Pure coordinate math. No external dependencies, no Bevy, no allocation
beyond `range` queries. Everything spatial in the project is built on
this crate.

Public API entry points: [`Coord`], [`MapBounds`], [`Direction`].

### `det_rng`

Deterministic-RNG primitives shared across the project. Exposes
`DetRng` (a newtype wrapper around `rand_chacha::ChaCha8Rng` with a
documented stability guarantee), `derive_seed` (combines a master seed
and a domain-specific salt into an independent sub-seed), and the
underlying `splitmix64` mixer.

The higher-level APIs that drive sub-RNG threading — `MasterRng`,
`StageRng`, and `StageId` for generation; the future `GameRng` for
runtime — live in their respective consumer crates because their
needs (stage-keyed vs system-keyed, one-shot vs save/load-aware)
diverge enough that a single API would serve neither well.

### `worldgen`

The generation pipeline: tectonics, climate, hydrology, biomes,
resources, starting positions. Takes a seed and [`WorldParams`],
produces [`WorldData`]. Runs standalone (used by both tools) and is
loaded by `game` at world creation time.

Stage 3.1 (tectonics) is complete. Stage 3.2 and beyond are next.

`TectonicsParams` (and all `WorldParams` tuning structs) derive
`serde::Serialize` / `serde::Deserialize` so they can be loaded from
RON preset files by `maptest` and `mapdump`.

`worldgen::debug_viz` contains shared colormap types (`BoundaryCmap`,
`HeightLevelCmap`, `Viridis`, `Categorical`, `Grayscale`) and
rasterization helpers used by both tools.

### `game`

The Bevy application. Owns the ECS world, plugin registration, systems,
and all runtime logic. Reads `WorldData` from `worldgen` to spawn
entities at game start.

Currently a stub; implementation begins in Stage 4.

### `mapdump` (tool)

A CLI binary for running the generation pipeline headlessly and dumping
intermediate and final fields to PNG for a single (seed, params) pair.
Accepts `--params <file.ron>` for parameter overrides; defaults to
`TectonicsParams::defaults()` when omitted. Used for quick single-seed
inspection; use `maptest` for multi-seed runs and parameter sweeps.

### `maptest` (tool)

A CLI binary for parameter tuning workflows that go beyond a single run:

- **`run`** — generate debug images for a fixed parameter set across
  multiple seeds (default: seeds 1–8) in sequence.
- **`sweep`** — vary one `TectonicsParams` scalar field across an
  evenly-spaced range of values, generating all (value, seed) pairs in
  parallel via `rayon`.

Both subcommands accept `--params <file.ron>` to load base parameters
from a RON preset file. Preset files live in `tools/maptest/presets/`.
See `tools/maptest/README.md` for full usage.

---

## Dependency graph
game     ──► coords \
──► worldgen ──► coords \
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└──► det_rng \
──► det_rng \
mapdump  ──► worldgen ──► (above) \
maptest  ──► worldgen ──► (above)

No cycles. `coords` and `det_rng` have zero project dependencies.
`worldgen` does not depend on `game`. `game` does not depend on
`mapdump`.

---

## Key architectural decisions

- **Explicit `MapBounds` parameter** — coordinate operations take
  `MapBounds` as an argument rather than `Coord` carrying a reference
  to its map. Avoids lifetimes on `Coord` and keeps the type `Copy`.

- **`Coord` is `Copy`** — two `i32` fields, no heap allocation, no
  `Drop`. Passes freely by value throughout the codebase.

- **`coords` has no runtime dependencies** — keeps compile times low
  and the crate usable outside the game (tests, tools, future server).

- **`worldgen` has no Bevy dependency** — generation runs headlessly.
  `mapdump` exists specifically to exploit this.

- **`det_rng` exposes only the shared primitive** — stage-keyed and
  system-keyed RNG APIs live in their respective consumers. See ADR
  for the rationale.

- **Parameter tuning via a dedicated `maptest` crate and RON preset
  files** — rather than CLI flags for every parameter or shell scripts,
  `TectonicsParams` is serializable/deserializable via `serde` + RON.
  Preset files in `tools/maptest/presets/` capture named configurations.
  `maptest sweep` runs parallel (value, seed) grids via `rayon`. This
  pattern scales to future generation stages without any tool changes.
  See ADR-0003 for the full rationale.

For the full decision history, see `documents/adr/`.