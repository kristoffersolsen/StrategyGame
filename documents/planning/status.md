# Implementation Status

**Date:** 2026-05-27  
**Stage:** 3.2 Climate — **complete**  
**Next action:** Stage 3.3 — Hydrology (design §1.11)

---

## Context

This project is a turn-based 2D strategy game inspired by Civilization,
built in Rust using the Bevy game engine. The codebase is in a Cargo
workspace with four library/game crates (`coords`, `worldgen`, `game`,
`det_rng`) and two CLI tools (`mapdump`, `maptest`). This document
covers the state of the `worldgen` crate and the generation pipeline
only.

Stages 0–3.2 are complete. Stage 3.3 (hydrology) is next.

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

---

## What is next

### Stage 3.3 — Hydrology (design doc §1.11)

Produces the river network, lake tiles, floodplain features, and canyon
features from the elevation and precipitation fields already computed.

**Cliff blocker resolved.** The blocker recorded at the Stage 3.2 gate
is closed. Design doc §1.4 already establishes that height delta ≥ 2
between adjacent tiles is impassable by default. Canyon walls are a
specific instance of this rule — no new design decision is required.
The resolution is documented in ADR-0012.

### Key decisions carried into implementation

- **D6 single-downstream flow direction** — one downstream neighbor per
  tile (lowest of six), deterministic tie-break by `Direction` enum
  ordinal. No multi-flow distribution. See ADR-0011.
- **Hybrid Planchon-Darboux sink resolution** — small basins
  (below `lake_basin_area_threshold`) are filled; large basins become
  lakes with a spill-point outlet. Priority queue over open tiles,
  `(r, q)` tie-break for equal-elevation tiles. See ADR-0011.
- **Precipitation-weighted flow accumulation** — each tile seeds its
  own `ClimateData::precipitation` value. Topological sort descending
  by elevation; single O(N) pass. See ADR-0011.
- **Post-hoc canyon carving** — runs after accumulation; `flow ×
  slope` threshold triggers height level reduction and canyon wall
  marking. Flow network not recomputed after carving (stable
  approximation). See ADR-0011.
- **`HydrologyData` owns a modified `height_level` copy** during the
  tuning period (Stage 3.3–3.4), to preserve both pre- and
  post-hydrology elevation fields as separate PNG outputs. Cleaned up
  at the Stage 3.4 gate. See ADR-0012.
- **Canyon walls** carry an `is_canyon_wall` flag. Impassability
  follows from the §1.4 delta ≥ 2 rule, not a new mechanism. See
  ADR-0012.
- **Fixed `(r, q)` iteration order** throughout; no parallelism in
  inner loops. Matches the climate stage determinism convention.

### Step roster

Steps are in dependency order; numbered `3.3.N` to match the mapdump
file prefix scheme (`33_NN_<name>.png`).

1. **Flow direction** — per-tile downstream neighbor. Output:
   `Field<Option<Direction>>` (`None` for ocean, coast, and lake
   terminals). Unit-testable on a hand-constructed 3×3 grid.
2. **Basin detection** — flood-fill from each sink tile; compute basin
   area and spill point elevation. Pure function over `(elevation,
   flow_direction, height_level)`.
3. **Sink resolution** — apply basin decisions: fill small basins
   (raise `elevation` and `height_level`), mark large basins as lake.
   Produces the post-hydrology `height_level` stored in
   `HydrologyData`.
4. **Flow accumulation** — topological sort by elevation, single-pass
   accumulation seeded by precipitation. Output: `Field<f32>` flow
   accumulator.
5. **River identification** — threshold accumulation → `RiverSize`
   enum (`Creek`, `River`, `MajorRiver`). Output:
   `Field<Option<RiverSize>>`.
6. **Floodplain marking** — adjacent-to-river tiles within
   `floodplain_elevation_tolerance` normalized elevation units. Output:
   `Field<bool>` floodplain feature.
7. **Canyon carving** — `flow × slope` threshold → lower `height_level`
   by `canyon_depth`, mark perpendicular neighbors as canyon walls.
   Output: modified `height_level` plus `Field<bool>` canyon wall
   feature.
8. **Hydrology stage entry point** — wires steps 1–7, constructs and
   returns `HydrologyData`. Follows climate stage pattern.
9. **Determinism tests** — extend existing scaffold to cover all
   hydrology output fields across multiple runs.

### File structure

```
crates/worldgen/src/stages/
└── hydrology.rs                ← run() entry point (step 8)
└── hydrology/
    ├── flow_direction.rs       ← step 1
    ├── basin.rs                ← steps 2–3 (detection + resolution)
    ├── accumulation.rs         ← step 4
    ├── rivers.rs               ← step 5
    ├── floodplain.rs           ← step 6
    └── canyon.rs               ← step 7
```

`world_data.rs` gains `HydrologyData` alongside `ClimateData`.
`params.rs` gains `params/hydrology.rs` with `HydrologyParams`,
serde-derived, validate method enforcing range constraints.

### Parameters

| Parameter | Default | Notes |
|-----------|---------|-------|
| `lake_basin_area_threshold` | 50 | Tile count; basins at or above this become lakes |
| `river_flow_threshold` | 0.15 | Normalized accumulated flow to become `Creek` |
| `major_river_flow_threshold` | 0.60 | Normalized accumulated flow to become `MajorRiver` |
| `floodplain_elevation_tolerance` | 0.05 | Normalized elevation delta from river tile |
| `canyon_flow_slope_threshold` | 0.25 | `flow × slope` product required for canyon carving |
| `canyon_depth` | 1 | Height levels removed; validated to range 1–2 |

All parameters exposed in `set_param` in `tools/maptest/src/sweep.rs`.
`HydrologyParams` added to `tools/maptest/presets/default.ron` with
per-field comments.

### PNG outputs

| File | Content |
|------|---------|
| `33_01_flow_direction.png` | Per-direction color overlay on elevation |
| `33_02_flow_accumulation.png` | Log-scale viridis accumulation field |
| `33_03_lakes.png` | Lake tiles overlaid on height-level map |
| `33_04_rivers.png` | River tiers overlaid on elevation |
| `33_05_floodplains.png` | Floodplain feature overlay |
| `33_06_canyons.png` | Canyon tiles and walls overlaid on elevation |

### Tuning targets

- **Every flow path terminates at ocean or lake** on seeds 1–8.
  Verified by an integration test that walks every tile's flow chain
  to its terminal.
- **Lake count is geographically plausible** — 5–20 per map at default
  threshold. Verified visually and by count assertion.
- **Canyon carving produces visible chokepoints** in high-slope river
  terrain without over-populating — typically 3–8 canyons per large
  continent at defaults.
- **Floodplain strips appear along major rivers.** Verifiable once
  biomes land in 3.4; the feature data must be correct here.
- **No determinism failures** across seeds 1–8.

### Key sweeps

1. `lake_basin_area_threshold` — controls lake count.
   Sweep 10–200 × 7 steps.
2. `river_flow_threshold` — controls river density.
   Sweep 0.05–0.40 × 7 steps.
3. `canyon_flow_slope_threshold` — controls canyon frequency.
   Sweep 0.10–0.50 × 6 steps.

### ADRs to write

- **ADR-0011:** Hydrology algorithm choices — D6 flow direction, hybrid
  Planchon-Darboux, precipitation-weighted accumulation, post-hoc
  canyon carving, no post-carve recomputation. **Written.**
- **ADR-0012:** Hydrology elevation ownership and canyon wall
  impassability — temporary `HydrologyData::height_level` copy,
  cleanup trigger at Stage 3.4 gate, canyon walls as application of
  §1.4 delta ≥ 2 rule. **Written.**

### Gate: Stage 3.3 complete

- All 9 steps implemented; `HydrologyData` populated by
  `generate_world`.
- `maptest run` and `mapdump` produce 6 new PNGs.
- Integration test confirms every flow path terminates at ocean or lake
  on seeds 1–8.
- Lake count is plausible on seeds 1–8 (visual + count assertion).
- Canyon carving produces visible chokepoints without over-populating.
- Determinism tests extended to cover all hydrology fields; pass across
  multiple runs.
- ADR-0011 and ADR-0012 written and accepted.
- `documents/planning/implementation-plan.md` updated to mark Stage 3.3
  complete and Stage 3.4 next.
- Design doc §1.11 updated with any decisions made during
  implementation that were left open in the design phase.

**Required before Stage 3.4 begins:** clean up `HydrologyData::height_level`
duplication per ADR-0012 cleanup condition. This is a gate prerequisite
for Stage 3.4, not a separate stage.

---

## Remaining pipeline stages

**Stage 3.4 — Biomes** (design doc §1.12)  
Whittaker-style (temperature, moisture) → biome lookup. Biome table as
RON config file. First cross-stage RON config use. Reads
post-hydrology `height_level` from `HydrologyData` (pre-cleanup) or
`TectonicData` (post-cleanup).

**Stage 3.5 — Resource placement** (design doc §1.13)  
Geological placement using plate mineral character, biome filtering,
clustering for luxuries, scattered for strategics.

**Stage 3.6 — Starting positions** (design doc §1.14)  
Resource-aware viability scoring, hard adequacy floor, spacing
constraints. Final exercise of the determinism scaffold.