# Architecture

This document describes the high-level structure of the codebase:
what the crates are, what they own, and how they depend on each other.
It is updated at each stage gate as new structure is added.

For *why* the architecture is this way, see `docs/design/overview.md`.
For *when* decisions were made, see `documents/adr/`.

**Current stage:** Stage 4.5 (Anti-corruption layer) complete; Stages 4, 3.6,
and rendering plan P1–P3 also complete.

---

## Workspace layout
crates/ \
content/    — content registry: string-id→handle mapping, used by worldgen and game \
coords/     — wrap-aware hexagonal coordinate math (no dependencies) \
det_rng/    — deterministic RNG primitives (no project dependencies) \
worldgen/   — procedural world generation pipeline (depends on coords, det_rng, content) \
game/       — Bevy application and runtime (depends on coords, worldgen, content) \
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

The game crate is divided into two trust tiers (see ADR trust-tier note in
`documents/planning/implementation_plan2.md`):

- **`sim` module (authoritative core).** The only tier permitted to mutate
  game state. Contains:
  - `StaticTerrain` — `game`'s owned static terrain resource. Built by
    consuming `WorldData` at startup via `from_world_data` (the anti-corruption
    layer, ADR-0025). Contains only runtime-relevant fields plus precomputed
    derivations (movement cost, `mountain_level`). `WorldData` is dropped
    immediately after translation; no `worldgen`-internal type persists.
  - `TileState` — mutable per-tile state as parallel `Field<T>` grids
    (owner, occupant, improvement, infrastructure, cleared). See ADR-0023.
  - `NetworkId` / `NetworkIdAllocator` / `EntityLookup` — deterministic
    cross-machine actor identity and the non-authoritative entity cache.
    See ADR-0024.
  - `YieldTable` — dense biome- and feature-indexed yield data loaded from
    `assets/worldgen/yield_table.ron` at startup.
  - `tile_yield(coord)` — the shared tile yield function combining static
    terrain and dynamic tile state.
  - `SimPlugin` — registers the above resources; `sim_startup` (in the
    `Startup` schedule) generates and translates the world, and `sim_init`
    (on `Running` entry) builds `TileState` and `YieldTable`.

- **`rendering` module (presentation tier).** Reads `StaticWorld` and
  future events from the core; never mutates game state. Contains the
  terrain chunk renderer, camera, lighting, and FPS overlay.

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
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;└──► content \
──► content \
mapdump  ──► worldgen ──► (above) \
maprender ─► game ──► worldgen ──► (above) \
maptest  ──► worldgen ──► (above)

No cycles. `coords`, `det_rng`, and `content` have zero project dependencies.
`worldgen` does not depend on `game`. `game` does not depend on `mapdump`.

Within `game`, the module dependency direction is `rendering` → `sim` (the
rendering module imports `sim::StaticWorld`); `sim` never imports from
`rendering`. This enforces the trust-tier rule: presentation depends on core,
not vice versa.

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

- **Anti-corruption layer at the `worldgen`→`game` boundary (ADR-0025)** —
  `game` owns `StaticTerrain`, shaped by what the runtime needs (not by
  `worldgen`'s internal layout). `WorldData` is consumed at startup and
  dropped immediately after translation, so `worldgen`'s internal types never
  appear in `game`'s runtime. This lets both crates evolve independently.
  Shared vocabulary (`Coord`, content handles) crosses the boundary as-is.

- **Dense-grid tile state, not per-tile entities (ADR-0023)** — mutable
  per-tile state lives in `TileState` as parallel `Field<T>` grids. Tiles
  are not ECS entities; only units and cities are entities. Rationale:
  at-scale archetype churn, O(1) coordinate access, and determinism.

- **`NetworkId` as authoritative cross-tile reference (ADR-0024)** — the
  tile grids store `Option<NetworkId>`, not `Option<Entity>`. Entity indices
  diverge across machines; `NetworkId` is derived from command-processing
  order and is identical everywhere.

- **`YieldTable` hot-path discipline (ADR-0022)** — biome string IDs are
  resolved to handles once at startup. All per-tile yield reads are O(1)
  array lookups indexed by handle; no string lookups at runtime.

- **`tile_yield(coord)` as the single shared yield function** — cities,
  worker-radius views, and the full-map yield overlay all call the same
  function. Its additive base (biome + features + resource) plus single
  improvement transform keeps the model simple and extensible.

For the full decision history, see `documents/adr/`.
