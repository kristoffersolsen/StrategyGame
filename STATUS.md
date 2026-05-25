# Implementation Status

**Date:** 2026-05-25  
**Stage:** 3.1 Tectonics — **complete**  
**Next action:** Stage 3.2 — Climate (design §1.10)

---

## Context

This project is a turn-based 2D strategy game inspired by Civilization,
built in Rust using the Bevy game engine. The codebase is in a Cargo
workspace with four library/game crates (`coords`, `worldgen`, `game`,
`det_rng`) and two CLI tools (`mapdump`, `maptest`). This document
covers the state of the `worldgen`
crate and the generation pipeline only.

Stages 0–3.1 are complete. Stage 3.2 (climate) is next.

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

### Stage 3.1 — Tectonics (complete)

All 9 steps in `crates/worldgen/src/stages/tectonics/`.

- Plate seed placement: Bridson Poisson disk sampling, wrap-aware;
  `PACKING_EFFICIENCY_FACTOR = 1.4` maps target count to sample radius
- Voronoi partition + noise-perturbed boundaries: every tile assigned
  to nearest seed; east-west wrap handled by torus-lifting the q-axis
  in 3D OpenSimplex noise
- Plate properties: `PlateType` (Continental/Oceanic, probabilistic per
  ADR-0002), `MotionVector` (random angle + magnitude), `MineralCharacter`
  (weighted draw over 5 geological categories)
- Boundary classification: Convergent/Divergent/Transform from approach
  velocity dot product; wrap-aware boundary normal
- Elevation field: per-plate base + intra-plate noise + boundary-effect
  Gaussian falloff + multi-octave fBm detail noise
- Sea level + quantile thresholding: elevation shifted to sea level = 0;
  `u8` height levels matched exactly to `level_proportions` by construction
- Hills: multi-source BFS distance field from boundary tiles; noise
  threshold interpolated between `hill_near_boundary_threshold` and
  `hill_interior_threshold`
- 8 PNG outputs per (seed, params) pair; 24-knob `TectonicsParams` with
  full rustdoc; `TectonicsParams` serde-serializable for RON preset files
- `tools/maptest` crate: `run` subcommand (multi-seed sequential) and
  `sweep` subcommand (parameter-range × seeds, parallel via `rayon`);
  `mapdump` simplified to 5 flags + `--params <file.ron>`; colormaps
  centralized in `worldgen::debug_viz`
- ADR-0003: parameter tuning infrastructure decision

---

## Current file structure

```
crates/worldgen/src/
├── lib.rs
├── error.rs
├── field.rs
├── params.rs                        ← WorldParams, TuningParams, etc.
├── params/
│   └── tectonics.rs                 ← TectonicsParams (24 tuning knobs, serde)
├── rng.rs                           ← MasterRng, StageRng, StageId
├── world_data.rs                    ← WorldData, TectonicData, Plate,
│                                      PlateType, MotionVector,
│                                      MineralCharacter, PlateId,
│                                      BoundaryType
├── debug_viz.rs                     ← PNG renderers + colormaps (Viridis,
│                                      Categorical, Grayscale, BoundaryCmap,
│                                      HeightLevelCmap)
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

tools/mapdump/src/
└── main.rs                          ← single-seed PNG dump (5 flags + --params)

tools/maptest/
├── presets/
│   └── default.ron                  ← reference preset, all TectonicsParams defaults
└── src/
    ├── main.rs                      ← run subcommand + shared CLI types
    ├── output.rs                    ← write_world_outputs, ensure_output_dirs
    └── sweep.rs                     ← sweep subcommand, set_param, linspace
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

## What is next

**Stage 3.2 — Climate** (design doc §1.10)

- Latitude-based base temperature field
- Wind field with latitude bands + noise
- Moisture advection (CFL-stable iterative scheme)
- Orographic precipitation (rain shadow behind mountains)
- Output: `Field<f32>` temperature and moisture fields on `WorldData`
- Add temperature, moisture, wind, precipitation PNG outputs to `output.rs` and `mapdump`
- Add climate scalar params to `set_param`; key sweeps: advection iterations, orographic strength, latitude falloff

**Blocker before Stage 3.3 (Hydrology):** resolve cliff impassability
question (design doc §1.4) before implementing canyon walls.

---

## Remaining pipeline stages

**Stage 3.3 — Hydrology** (design doc §1.11)  
Flow accumulation, sink resolution, lake detection, river identification,
floodplain marking, canyon carving.

**Stage 3.4 — Biomes** (design doc §1.12)  
Whittaker-style (temperature, moisture) → biome lookup. Biome table as
RON config file.

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
- **PNG output for every step.** Each step that produces a new field
  gets a PNG output in both `mapdump` (single-seed) and
  `tools/maptest/src/output.rs` (multi-seed / sweep). Naming convention:
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
