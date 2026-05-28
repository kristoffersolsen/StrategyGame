# Architecture

This document describes the high-level structure of the codebase:
what the crates are, what they own, and how they depend on each other.
It is updated at each stage gate as new structure is added.

For *why* the architecture is this way, see `docs/design/overview.md`.
For *when* decisions were made, see `documents/adr/`.

**Current stage:** Stage P3.1 map rendering implemented; full P3 scene
work continues.

---

## Workspace layout
crates/ \
coords/     — wrap-aware hexagonal coordinate math (no dependencies) \
det_rng/    — deterministic RNG primitives (no project dependencies) \
worldgen/   — procedural world generation pipeline (depends on coords, det_rng) \
game/       — Bevy application and runtime (depends on coords, worldgen) \
tools/ \
mapdump/    — CLI: dump a single seed's generation output to PNG \
maprender/  — Bevy app: interactive map rendering performance gate \
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

Stages 3.1 through 3.5 are complete. Starting locations are still
deferred.

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

Stage P1/P2/P3.1 adds the static map rendering foundation:

- `RenderingPlugin` registered from `main.rs`.
- Chunked 16x16 pointy-top hex terrain meshes built from `WorldData`.
- Vertex-colour terrain lookup for the P1 baseline.
- Texture-array terrain renderer for the P2 baseline, using a custom
  Bevy 2D material, WGSL shader, per-vertex texture indices, and 64x64
  generated placeholder texture layers.
- `RenderAppState`, a minimal loading/spawning/running sequence that
  prepares renderer assets before terrain chunks spawn.
- `RenderStartupConfig`, which selects world-generation parameters and
  terrain renderer implementation at startup.
- P3.1/P3.2 3D scene pivot through `TerrainRenderer::Terrain3d`, using
  `Mesh3d`, `MeshMaterial3d<StandardMaterial>`, a perspective
  `Camera3d`, directional light, ambient light, sky clear color,
  elevation mesh chunks, side faces, mountain peaks, and a water plane.
- `TerrainRenderer`, currently supporting `VertexColor`, `TextureArray`,
  and `Terrain3d`.
- A `Camera2d` map camera with WASD/arrow-key pan and scroll-wheel zoom.
- A P3 orbit `Camera3d` view for the 3D renderer, with pivot pan,
  scroll zoom, right-drag yaw/pitch, and keyboard pan relative to camera
  yaw.
- A Bevy diagnostics text overlay showing FPS and frame time.

The production binary currently inserts the default render startup
configuration. Render performance gates and parameter edits are owned by
the `maprender` tool, not by hardcoded game startup.

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

### `maprender` (tool)

A Bevy executable for interactive terrain-rendering performance gates.
It uses the reusable `game::rendering` plugin and loads a RON preset
from `tools/maprender/presets/`. Presets select seed, map size, terrain
renderer implementation, and `worldgen::TuningParams`.

P1 uses `renderer: VertexColor`. P2 uses `renderer: TextureArray`.
P3.1 uses `renderer: Terrain3d`. All renderers are selected through the
same preset field so test runs can switch rendering pipelines without
changing the game crate startup architecture.

---

## Dependency graph
game     ──► coords \
──► worldgen ──► coords \
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└──► det_rng \
mapdump  ──► worldgen ──► (above) \
maprender ─► game ──► worldgen ──► (above) \
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

- **Chunked static terrain rendering** — the game crate renders generated
  terrain as one 2D mesh entity per 16x16 tile chunk. This keeps entity
  count and draw calls low while preserving a future rebuild boundary for
  terrain changes. See ADR-0018.

- **3D scene pivot is additive during P3** — P1/P2 renderers remain
  available for baseline comparison while P3 adds the `Terrain3d`
  renderer behind the same startup configuration path.

- **Rendering gates live in tool crates** — renderer performance tests
  run through `tools/maprender` with RON presets. The game crate exposes
  reusable rendering plugins and startup configuration rather than
  hardcoding test worlds in production startup.

For the full decision history, see `documents/adr/`.
