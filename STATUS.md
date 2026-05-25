# Implementation Status

**Date:** 2026-05-25  
**Stage:** 3.1 Tectonics — **complete** (all 9 steps)  
**Next action:** Stage 3.2 — Climate (design §1.10)

---

## Context

This project is a turn-based 2D strategy game inspired by Civilization,
built in Rust using the Bevy game engine. The codebase is in a Cargo
workspace with four crates: `coords`, `worldgen`, `game`, and the
`mapdump` CLI tool. This document covers the state of the `worldgen`
crate and the generation pipeline only.

Stages 0 (foundations), 1 (coords crate), and 2 (worldgen
infrastructure) are complete. Stage 3 (the generation pipeline) is in
progress at sub-stage 3.2 (climate).

---

## Completed work

### Stages 0–2 (complete)

- Workspace skeleton, CI, git hooks, linting, rustfmt
- `coords` crate: `Coord` (axial), `MapBounds` (wrap-aware),
  hex distance, normalization — fully tested
- `worldgen` infrastructure: `Field<T>`, `WorldParams`, `WorldData`,
  `GenerationError`, `MasterRng`/`StageRng` deterministic RNG threading,
  `debug_viz` PNG dump module (pixel and hex renderers, colormaps),
  `mapdump` CLI tool
- Determinism test scaffold (placeholder hashes, to be activated
  when the full pipeline produces stable output)
- ADRs: `0001-infra-determinism-and-rng-threading.md`,
  `0002-worldgen-probabilistic-plate-type-assignment.md`

### Stage 3.1 — Tectonics (complete, all 9 steps)

All implemented in `crates/worldgen/src/stages/tectonics/`.

**Step 1 — Plate seed placement** (`seeds.rs`)  
Bridson's Poisson disk sampling, wrap-aware via `MapBounds::distance`.
Radius derived from target plate count and map area via
`PACKING_EFFICIENCY_FACTOR = 1.4` (empirically calibrated).
Produces ~9–11 seeds at default target of 10.

**Steps 2–3 — Voronoi partition + boundary perturbation** (`voronoi.rs`,
`perturbation.rs`)  
Every tile assigned to nearest seed by hex distance. Boundaries
perturbed by wrap-aware 3D OpenSimplex noise (q-axis lifted onto torus
circle for seamless east-west wrapping). `boundary_perturbation_amplitude`
controls waviness (default 8.0 tiles).

**Step 4 — Plate properties** (`plates.rs`)  
Each plate assigned:
- `PlateType` (Continental/Oceanic) — probabilistic from `land_fraction`
  implied by `level_proportions` (ADR-0002)
- `MotionVector` — random angle + magnitude in
  `[motion_magnitude_min, motion_magnitude_max]`
- `MineralCharacter` — weighted random draw over 5 geological categories

**Step 5 — Boundary classification** (`boundary.rs`)  
Per-tile boundary type (`None` = interior, `Some(Convergent/Divergent/Transform)`).
Computed from approach velocity: `dot(motion_A − motion_B, normal_A→B)`.
`convergence_threshold` separates convergent/divergent from transform.
Wrap-aware boundary normal using the shorter east-west arc.

**Steps 6–7 — Elevation field** (`elevation.rs`)  
Continuous `Field<f32>`. Three additive contributions:
1. Per-plate base (`continental_base_elevation` / `oceanic_base_elevation`)
2. Intra-plate OpenSimplex noise (low amplitude)
3. Boundary effects: modifier from `(own type, adj type, boundary type)`
   lookup, propagated via Gaussian falloff (`boundary_falloff_sigma = 8` tiles).
Then multi-octave fBm detail noise overlaid uniformly (step 7).

**Step 8 — Sea level + quantile thresholding** (`sea_level.rs`)  
Elevation field shifted so sea level = 0. Each tile bucketed into a
`u8` height level by quantile thresholds derived from `level_proportions`.
Achieved proportions match `level_proportions` exactly by construction.

**Step 9 — Hills classification** (`hills.rs`)  
Per-tile `bool` hilliness flag. Multi-source BFS from boundary tiles
produces a distance field; threshold linearly interpolates between
`hill_near_boundary_threshold` (near boundaries) and
`hill_interior_threshold` (far interior). Tile is hilly if OpenSimplex
noise exceeds the interpolated threshold.

**Visual output verified** (via `mapdump` + `scripts/test_seeds.sh`):
- `31_01_plate_seeds_px.png` — seed scatter, even distribution
- `31_03_plate_map_px.png` / `hex` — organic plate boundaries
- `31_05_boundary_types_px.png` — convergent (red) / divergent (blue) / transform (grey)
- `31_07_elevation_px.png` — continuous elevation field (Viridis)
- `31_08_height_levels_px.png` / `hex` — discrete 6-band height classification
- `31_09_hills_px.png` — hilliness overlay (white = hilly)

---

## Current file structure (worldgen crate)

```
crates/worldgen/src/
├── lib.rs
├── error.rs
├── field.rs
├── params.rs                        ← WorldParams, TuningParams, etc.
├── params/
│   └── tectonics.rs                 ← TectonicsParams (all tuning knobs)
├── rng.rs                           ← MasterRng, StageRng, StageId
├── world_data.rs                    ← WorldData, TectonicData, Plate,
│                                      PlateType, MotionVector,
│                                      MineralCharacter, PlateId,
│                                      BoundaryType
├── debug_viz.rs                     ← PNG dump functions and colormaps
├── stages.rs
└── stages/
    └── tectonics.rs                 ← run() entry point (all 9 steps)
    └── tectonics/
        ├── seeds.rs                 ← step 1: Poisson disk
        ├── voronoi.rs               ← step 2: Voronoi partition
        ├── perturbation.rs          ← step 3: noise perturbation
        ├── plates.rs                ← step 4: plate properties
        ├── boundary.rs              ← step 5: boundary classification
        ├── elevation.rs             ← steps 6–7: elevation + detail noise
        ├── sea_level.rs             ← step 8: quantize to height levels
        └── hills.rs                 ← step 9: hilliness classification
```

---

## Tuning parameters (TectonicsParams) — current defaults

All live in `crates/worldgen/src/params/tectonics.rs`.

| Parameter | Default | Notes |
|---|---|---|
| `target_plate_count` | 10 | Achieved count ~9–11 |
| `boundary_perturbation_amplitude` | 8.0 tiles | Controls boundary waviness |
| `motion_magnitude_min` | 0.5 | Dimensionless |
| `motion_magnitude_max` | 1.5 | Dimensionless |
| `land_level_count` | 3 | Low / mid / high land |
| `level_proportions` | `[0.35, 0.05, 0.25, 0.20, 0.10, 0.05]` | Ocean, coast, land×3, mountain |
| `mineral_weights` | `[4.0, 2.0, 1.0, 3.0, 4.0]` | Ferrous, Precious, Gems, Hydrocarbons, Industrial |
| `convergence_threshold` | 0.3 | Dot-product cutoff for convergent/divergent vs transform |
| `continental_base_elevation` | 0.3 | Base elevation for continental plates |
| `oceanic_base_elevation` | −0.3 | Base elevation for oceanic plates |
| `intra_plate_noise_amplitude` | 0.05 | Interior texture noise amplitude |
| `boundary_falloff_sigma` | 8.0 tiles | Gaussian σ for boundary-effect spread |
| `mountain_elevation_boost` | 1.2 | Con–Con convergent |
| `coastal_mountain_boost` | 0.8 | Con–Oce convergent (continental side) |
| `subduction_trench_penalty` | −0.8 | Oce–Con convergent (oceanic side) |
| `island_arc_boost` | 0.4 | Oce–Oce convergent |
| `rift_valley_penalty` | −0.4 | Con–Con divergent |
| `mid_ocean_ridge_boost` | 0.3 | Oce–Oce divergent |
| `detail_noise_amplitude` | 0.08 | fBm overlay amplitude |
| `detail_noise_frequency` | 4.0 | fBm base frequency |
| `detail_noise_octaves` | 3 | fBm octave count |
| `hill_influence_radius` | 8 tiles | BFS radius from boundary |
| `hill_near_boundary_threshold` | −0.2 | Noise threshold at boundary (≈60 % hilly) |
| `hill_interior_threshold` | 0.55 | Noise threshold far from boundary (≈22 % hilly) |
| `hill_noise_frequency` | 3.0 | Spatial frequency of hill noise |

---

## What is next: Stage 3.2 — Climate

Per design doc §1.10 and `IMPLEMENTATION_PLAN.md`:

- Latitude-based base temperature field
- Wind field with latitude bands + noise
- Moisture advection (CFL-stable iterative scheme)
- Orographic precipitation (rain shadow behind mountains)
- Output: `Field<f32>` temperature and moisture fields on `WorldData`

**Blocker before Stage 3.3 (Hydrology):** resolve cliff impassability
question (design doc §1.4) before implementing canyon walls.

---

## After Stage 3.1: remaining pipeline stages

Per `IMPLEMENTATION_PLAN.md`:

**Stage 3.2 — Climate** (design doc §1.10)  
Latitude-based temperature + advection-simulated moisture. Wind field
with latitude bands + noise. Orographic precipitation (rain shadows
behind mountains). Most numerically delicate stage — CFL stability
matters.

**Stage 3.3 — Hydrology** (design doc §1.11)  
Flow accumulation, sink resolution, lake detection, river
identification, floodplain marking, canyon carving.  
**Blocker:** Resolve cliff impassability question (design doc §1.4)
before implementing canyon walls.

**Stage 3.4 — Biomes** (design doc §1.12)  
Whittaker-style (temperature, moisture) → biome lookup. RON config
file for biome table (first use of RON in the project).

**Stage 3.5 — Resource placement** (design doc §1.13)  
Geological placement using plate mineral character, biome filtering,
clustering for luxuries, scattered for strategics.

**Stage 3.6 — Starting position selection** (design doc §1.14)  
Resource-aware viability scoring, spacing constraints, adequacy floor,
optional audit pass. Final determinism test exercise.

---

## Key conventions to maintain

- **No magic numbers.** Every tunable is a named constant in
  `TectonicsParams` (or the relevant stage params struct) with full
  rustdoc: what it controls, sensible range, what happens at extremes,
  default and rationale.
- **Parameter sanity sweep before moving on.** For each new tuning
  parameter: test it at low / default / high, confirm the visible
  effect is monotonic and expected. This is the invariant check, not
  an aesthetic tuning pass.
- **`mapdump` output for every step.** Each step that produces a new
  field gets a PNG output in `mapdump`. Naming convention:
  `31_05_boundary_types_px.png`, `31_07_elevation_px.png`, etc.
- **Wrap-awareness everywhere.** Distance checks, noise evaluation,
  and BFS must all use `MapBounds::distance` or the torus-lifted noise
  pattern from `perturbation.rs`. A seam artifact at the east-west
  boundary is always a wrap bug.
- **Determinism.** All RNG flows through `StageRng`. No `thread_rng`.
  No non-deterministic iteration order affecting output. The
  determinism tests in `crates/worldgen/tests/determinism.rs` should
  be un-ignored and activated with real reference hashes once the full
  pipeline is stable.
- **Style guide and documentation guide** must be read before writing
  new modules (`STYLE_GUIDE.md`, `DOCUMENTATION_GUIDE.md` at repo
  root).
