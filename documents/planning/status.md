# Implementation Status

**Date:** 2026-05-27  
**Stage:** 3.3 Hydrology — **complete**  
**Next action:** Stage 3.4 — Biomes (design §1.12)

---

## Context

This project is a turn-based 2D strategy game inspired by Civilization,
built in Rust using the Bevy game engine. The codebase is in a Cargo
workspace with four library/game crates (`coords`, `worldgen`, `game`,
`det_rng`) and two CLI tools (`mapdump`, `maptest`). This document
covers the state of the `worldgen` crate and the generation pipeline
only.

Stages 0–3.3 are complete. Stage 3.4 (biomes) is next.

---

## Completed work

### Stages 0–2 (complete)

- Workspace skeleton, CI, git hooks, linting, rustfmt.
- `coords` crate: `Coord` (axial), `MapBounds` (wrap-aware),
  hex distance, normalization — fully tested.
- `worldgen` infrastructure: `Field<T>`, `WorldParams`, `WorldData`,
  `GenerationError`, `MasterRng`/`StageRng` deterministic RNG threading,
  `debug_viz` PNG dump module (pixel and hex renderers, colormaps),
  `mapdump` CLI tool.
- Determinism test scaffold (placeholder hashes, to be activated
  when the full pipeline produces stable output).
- ADRs: `0001`–`0007` foundational decisions.

### Stage 3.1 — Tectonics (complete)

All 10 steps in `crates/worldgen/src/stages/tectonics/`.

- Plate generation (crystal-growth BFS), boundary classification,
  base elevation, sea level, discrete height levels, hill pass,
  mountain pass carving.
- `TectonicData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 8 PNG outputs.
- ADR-0008 through ADR-0010 written and accepted.

### Stage 3.2 — Climate (complete)

All 9 steps in `crates/worldgen/src/stages/climate/`.

Key decisions carried into implementation:

- **Conservative donor-cell upwind discretisation** of
  `∂M/∂t + ∇·(vM) = S − P`. Wind projections computed in Cartesian
  hex space; six hex neighbour unit vectors spaced 60° apart from east.
- **CFL hard cap** `|v| ≤ 0.25` per tile per iteration. Validated at
  parameter load time.
- **Smooth wind profile** `u(φ) = −A·cos(3π·φ)` plus low-frequency
  noise perturbation, producing three latitude bands without
  discontinuities at transitions.
- **Three-phase iteration:** source → advection → precipitation, with
  double-buffered fields.
- **Per-tile orographic precipitation:** `Δelev` from the single upwind
  neighbour, not per-deposit accounting.
- **Fixed `(r, q)` lexicographic iteration order** for determinism; no
  parallelism in the advection loop.
- **Iteration count scales with map width** — default `≈ 1.5 ×
  map_width`.
- **All water tiles are moisture sources**, not just `ocean`.

Gate conditions met:

- `ClimateData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 7 new PNGs across four output
  categories.
- Visible rain shadows behind mountain chains on seeds 1–8.
- Latitude bands clearly visible in precipitation map: wet tropics,
  dry subtropics, wet mid-latitudes, dry poles.
- No horizontal stripe artefacts at default `wind_noise_amplitude`.
- CFL validation catches a deliberately-bad `wind_amplitude = 0.3`
  parameter at load time.
- Determinism tests pass across multiple runs for all four climate
  fields.
- ADR-0004 and ADR-0005 written and accepted.

### Stage 3.3 — Hydrology (complete)

All 9 steps in `crates/worldgen/src/stages/hydrology/`.

Key decisions carried into implementation:

- D6 single-downstream flow direction with hydrology-local tie-break
  order matching ADR-0011.
- Hybrid sink resolution: small basins are filled with a tiny routing
  gradient toward the spill point; large basins become lake terminals.
- Precipitation-weighted flow accumulation normalised by maximum land
  flow for map-size-independent river thresholds.
- River tiers, floodplains, canyon floors, canyon walls, and temporary
  post-hydrology height levels are stored in `HydrologyData`.
- `mapdump` and `maptest` emit the six Stage 3.3 PNG outputs.

Gate conditions met:

- `HydrologyData` populated by `generate_world`.
- Integration test confirms every flow path terminates at water or lake
  for seeds 1–8.
- Determinism hash covers all hydrology output fields.
- Hydrology tuning params are available in `TuningParams`, `set_param`,
  and `tools/maptest/presets/default.ron`.

---

## What is next

### Stage 3.4 — Biomes (design doc §1.12)

Produces biome assignments from temperature, moisture, elevation, and
hydrology outputs.

**Cliff blocker resolved.** The blocker recorded at the Stage 3.2 gate
is closed. Design doc §1.4 already establishes that height delta ≥ 2
between adjacent tiles is impassable by default. Canyon walls are a
specific instance of this rule — no new design decision is required.
The resolution is documented in ADR-0012.

### Key decisions carried into implementation

- Whittaker-style 2D lookup from temperature and moisture.
- Hydrology overrides for rivers, lakes, and floodplains.
- Downstream code must read `HydrologyData::height_level` during the
  Stage 3.4 tuning period; see ADR-0012 cleanup condition.

### Step roster

Steps are in dependency order; numbered `3.4.N` to match the mapdump
file prefix scheme (`34_NN_<name>.png`).

1. **Biome taxonomy** — define biome enum and lookup-table RON schema.
2. **Climate normalization** — derive biome input coordinates from
   temperature and moisture/precipitation fields.
3. **Base lookup** — assign biome from the Whittaker-style table.
4. **Hydrology overrides** — rivers, lake adjacency, and floodplains
   override toward wetter/fertile variants.
5. **Elevation overrides** — mountains, highlands, and post-hydrology
   height levels constrain biome choices.
6. **Transition smoothing** — optional local smoothing if the base
   lookup produces blocky edges.
7. **Biome stage entry point** — constructs and returns `BiomeData`.
8. **Determinism tests** — extend the hash scaffold to cover biome
   output fields.

### File structure

Expected shape:

```
crates/worldgen/src/stages/
└── biomes.rs
└── biomes/
    ├── table.rs
    ├── lookup.rs
    ├── overrides.rs
    └── smoothing.rs
```

### PNG outputs

| File | Content |
|------|---------|
| `34_01_biomes.png` | Categorical biome map |
| `34_02_biome_inputs.png` | Temperature/moisture diagnostic if useful |
| `34_03_hydrology_overrides.png` | Tiles changed by hydrology overrides |

### Tuning targets

- Biome transitions look natural rather than blocky.
- No unhandled temperature/moisture combinations produce undefined
  biomes.
- Hydrology overrides create fertile river/floodplain strips through
  otherwise dry regions.
- No determinism failures across seeds 1–8.

**Required at the Stage 3.4 gate:** clean up
`HydrologyData::height_level` duplication per ADR-0012 before moving
to Stage 3.5.

---

## Remaining pipeline stages

**Stage 3.5 — Resource placement** (design doc §1.13)  
Geological placement using plate mineral character, biome filtering,
clustering for luxuries, scattered for strategics.

**Stage 3.6 — Starting positions** (design doc §1.14)  
Resource-aware viability scoring, hard adequacy floor, spacing
constraints. Final exercise of the determinism scaffold.
