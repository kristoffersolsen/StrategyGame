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
covers the state of the `worldgen` crate and the generation pipeline
only.

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

(Existing Stage 3.1 summary, file structure, parameters table, and
default values are unchanged. See the prior version of this document
for the full Stage 3.1 reference; the plan below assumes that work
landed and is in place.)

---

## What is next

**Stage 3.2 — Climate** (design doc §1.10)

The most numerically delicate stage in the pipeline. Produces two
output fields — temperature and moisture — plus a precipitation
accumulator and a wind field, by simulating moisture advection on the
hex grid to steady state under a latitude-banded wind field with
orographic precipitation.

The mathematical model and hex-grid discretisation were worked out in
the Stage 3.2 planning session and are summarised in
`docs/design/overview.md` §1.10. Key points carried into
implementation:

- **Conservative donor-cell upwind discretisation** of
  `∂M/∂t + ∇·(vM) = S − P`. Wind projections computed in Cartesian
  hex space (`sqrt(3)`/`3/2` axial-to-Cartesian conversion); the six
  hex neighbour unit vectors are spaced 60° apart starting from east.
- **CFL hard cap** `|v| ≤ 0.25` per tile per iteration (safety
  margin under the strict `2|v| ≤ 1` bound). Validated at parameter
  load time, not at run time.
- **Smooth wind profile** `u(φ) = −A·cos(3π·φ)` plus low-frequency
  noise perturbation. The single cosine produces the three latitude
  bands without a discontinuity at the band transitions. (Marked as
  a candidate for revision if visible artefacts appear.)
- **Three-phase iteration:** source → advection → precipitation,
  in that order, with double-buffered fields. Precipitation runs as
  a separate pass over the post-advection field, not inline during
  the advection sweep.
- **Per-tile orographic precipitation:** `Δelev` is the elevation
  gain from the upwind neighbour (one neighbour, defined by wind
  direction at the destination tile), not per-deposit accounting.
- **Fixed iteration order** (`(r, q)` lexicographic) for
  floating-point determinism; no parallelism in the advection loop.
- **Iteration count scales with map width** — default
  `≈ 1.5 × map_width` rather than a fixed constant, so moisture has
  time to propagate across large maps.
- **East-west wrap** natural via `MapBounds::normalize`. North-south
  is a hard boundary; moisture transferred off the top or bottom
  row is lost from the system (correct behaviour for poles).
- **All water tiles are moisture sources**, not just `ocean`. Inland
  seas evaporate.

### Step roster

The stage breaks into nine steps, mirroring the Stage 3.1 pattern.
Each step is independently PR-able and visually verifiable. Steps are
listed in dependency order; numbering is `3.2.N` to match the
mapdump file prefix scheme (`32_NN_<name>.png`).

1. **Latitude-based temperature field** — cosine in latitude plus
   elevation lapse rate. Output: `Field<f32>` temperature in
   normalised units. Independent of moisture; can be tested in
   isolation.
2. **Wind field construction** — smooth cosine zonal profile plus
   low-frequency noise perturbation. Output: `Field<Vec2>` wind
   velocity. CFL validation at construction time.
3. **Evaporation source field** — per-tile evaporation rate from
   water tiles (ocean, coast, future lakes), scaled by temperature.
   Output: `Field<f32>` evaporation rate.
4. **Advection step kernel** — single-iteration sweep: outflow
   weights, distribute to neighbours in fixed order, retain
   `(1 − W)` fraction, add source. Pure function over
   `(M_prev, wind, evap)` → `M_next`. Unit-testable against
   hand-constructed inputs.
5. **Precipitation step kernel** — single sweep over `M_next`
   subtracting precipitated moisture and accumulating into a
   separate precipitation field. Pure function over
   `(M, temp, wind, elevation)` → `(M_after, precip_delta)`.
6. **Iteration driver** — runs steps 4–5 in a loop for the
   configured iteration count, double-buffering. No per-iteration
   convergence check; fixed count for determinism.
7. **Climate stage entry point** — wires steps 1–6 together,
   threads the stage RNG, returns the populated `ClimateData`.
8. **Boundary correctness audit** — verify pole behaviour
   (moisture leaving polar rows is lost), wrap behaviour
   (east-west continuity), and inland-sea behaviour (non-ocean
   water tiles act as sources). Lightweight integration tests
   plus visual confirmation.
9. **Determinism tests** — same seed and params produce
   byte-identical temperature, moisture, wind, and precipitation
   fields across runs. Activated via the existing determinism
   scaffold.

### File structure

Add under `crates/worldgen/src/stages/`:

```
stages/
└── climate.rs                       ← run() entry point (steps 1–7)
└── climate/
    ├── temperature.rs               ← step 1
    ├── wind.rs                      ← step 2
    ├── evaporation.rs               ← step 3
    ├── advection.rs                 ← step 4
    ├── precipitation.rs             ← step 5
    └── iterate.rs                   ← step 6
```

`world_data.rs` gains a `ClimateData` sub-struct alongside
`TectonicData`, carrying `temperature`, `moisture`, `wind`, and
`precipitation` fields plus their `empty(bounds)` constructor.

`params.rs` gains a `params/climate.rs` module exporting
`ClimateParams`, added as `TuningParams::climate`. Fields rustdoc'd
individually, `serde::{Serialize, Deserialize}` derived, `validate`
method enforces the CFL cap and other hard bounds.

`lib.rs` adds the climate stage call between tectonics and the
hydrology placeholder, threading `StageRng` with a new `StageId`
variant `Climate`.

### Parameters

Initial defaults to lock in during tuning. Hard bounds enforced by
`validate`; recommended ranges enforced by the maptest preset
defaults.

| Parameter | Default | Notes |
|---|---|---|
| `equator_temperature` | 1.0 | Normalised; equator baseline. |
| `pole_temperature` | −0.5 | Normalised; pole baseline. |
| `elevation_lapse_rate` | 0.25 | Temperature drop per unit elevation. |
| `climate_band_offset` | 0.0 | Global temperature shift from advanced layer. |
| `wind_amplitude` | 0.18 | Zonal wind amplitude `A`. Hard cap: 0.25 (CFL). |
| `wind_noise_amplitude` | 0.04 | Low-frequency perturbation magnitude. |
| `wind_noise_frequency` | 0.5 | Perturbation spatial frequency (map-width units). |
| `evaporation_rate` | 0.05 | Per-iteration source from water tiles. |
| `evaporation_temp_floor` | −0.2 | Temperature below which evaporation is zero. |
| `evaporation_temp_scale` | 1.0 | Linear factor scaling evaporation with temperature. |
| `base_precipitation_rate` | 0.02 | Fraction precipitated per iteration in flat terrain. |
| `temp_capacity_offset` | 0.5 | Air capacity sigmoid offset; lower → more cold rain. |
| `temp_capacity_scale` | 4.0 | Air capacity sigmoid steepness. |
| `orographic_factor` | 1.5 | Multiplier on positive `Δelev_uphill`. |
| `max_precipitation_fraction` | 0.6 | Hard cap on per-iteration precipitation fraction. |
| `iteration_count_factor` | 1.5 | Multiplier on `map_width` to get iteration count. |
| `iteration_count_min` | 100 | Floor for small maps. |

`iteration_count` is derived, not stored: `max(iteration_count_min,
round(iteration_count_factor * map_width))`. Exposed in maptest as
the underlying factor, since the derived count is what the user
actually wants to vary.

### PNG outputs (mapdump and maptest)

Stage 3.2 adds five new PNG types, prefix `32_`:

- `32_01_temperature_px.png` — viridis temperature field.
- `32_02_wind_field_px.png` — wind direction shown as a hue/value
  encoding per tile (hue = direction, value = magnitude). One pixel
  per tile.
- `32_03_evaporation_px.png` — viridis evaporation source field.
- `32_06_moisture_px.png` — viridis steady-state moisture field.
- `32_06_precipitation_px.png` — viridis accumulated precipitation
  field. The visual headline output for tuning — rain shadows
  show up here.

Each gets a hex-rasterized companion (`_hex`) for the two outputs
where individual tiles matter most: temperature and precipitation.
Total new outputs: 7. Add the matching hue-direction colormap to
`worldgen::debug_viz` (`DirectionCmap`).

### Tuning integration

- Add `ClimateParams` match arms to `set_param` in
  `tools/maptest/src/sweep.rs`. All 16 scalar fields above are
  sweepable.
- Add image-writing calls for the 7 new outputs to
  `tools/maptest/src/output.rs` → `write_world_outputs`.
- Extend `tools/maptest/presets/default.ron` to include
  `ClimateParams` with every field at its default and a
  per-field comment.
- Update `tools/maptest/README.md` sweepable-fields list and
  output-files table.
- Update `scripts/test_seeds.sh` to copy the new files into per-type
  subdirectories under `tmp/tests/mapgen/`.

### Tuning targets

Per design doc §1.10 risks/mitigations:

- **Visible rain shadows** behind mountain chains. The orographic
  factor is the headline knob.
- **Latitude bands match expectations** — wet tropics, dry
  subtropics, wet mid-latitudes, dry poles.
- **No horizontal stripe artefacts.** If they appear, increase
  `wind_noise_amplitude` first; revisit the smooth profile only if
  that fails.
- **CFL stability holds.** No NaN, no unbounded growth. The
  validate-time CFL check should prevent this; a runtime assertion
  in debug builds catches programming errors that bypass it.
- **Inland seas are wet sources.** Visually verify that landlocked
  ocean tiles produce moisture downwind.
- **Pole regions are dry.** Acceptable v1 behaviour; not a bug.

### Key sweeps

The parameters most worth sweeping first, in priority order:

1. `orographic_factor` — controls rain-shadow intensity. Sweep
   `0.5..3.0` × 7 steps.
2. `iteration_count_factor` — verifies steady-state convergence.
   Sweep `0.8..2.5` × 5 steps; field should change little above
   ~1.2.
3. `base_precipitation_rate` — global moisture distribution.
   Sweep `0.005..0.05` × 6 steps.
4. `wind_amplitude` — must stay below CFL cap. Sweep
   `0.05..0.24` × 5 steps. Verify no stripe artefacts at any value.
5. `wind_noise_amplitude` — sweep `0.0..0.1` × 5 steps. Confirms
   noise removes stripes.
6. `evaporation_rate` and `equator_temperature` — second-pass tuning
   once orographic and precipitation defaults are settled.

### ADRs to write

- **ADR-0004 (proposed): Climate stage iteration count and
  determinism strategy.** Records that iteration count is fixed
  (not convergence-checked), tied to map width by a configurable
  factor, and that the advection loop is single-threaded with fixed
  `(r, q)` iteration order. Cross-references the determinism ADR
  and the maptest infrastructure ADR.
- **ADR-0005 (proposed): Hex grid advection discretisation.**
  Records the choice of donor-cell upwind in Cartesian-projected
  hex space, the `|v| ≤ 0.25` CFL bound derivation, the separate
  precipitation pass, and the per-tile orographic semantics.
  Alternatives considered: semi-Lagrangian advection (rejected for
  complexity), inline precipitation (rejected for order-dependence),
  per-deposit orographic accounting (rejected for over-complication
  without physical accuracy gain).

### Gate: Stage 3.2 complete

- All 9 steps implemented; `ClimateData` populated by
  `generate_world`.
- `maptest run` and `mapdump` produce 7 new PNGs across the four
  output categories.
- A range of seeds shows visible rain shadows behind mountains
  consistently.
- Latitude bands are clearly visible in the precipitation map: wet
  tropics, dry subtropics, wet mid-latitudes, dry poles.
- No horizontal stripe artefacts at default `wind_noise_amplitude`.
- CFL validation catches a deliberately-bad
  `wind_amplitude = 0.3` parameter at load time.
- Determinism test extended to cover all four climate fields;
  passes across multiple runs.
- ADR-0004 and ADR-0005 written and accepted.
- `IMPLEMENTATION_PLAN.md` updated to mark Stage 3.2 complete and
  Stage 3.3 next.
- Design doc §1.10 updated with any decisions made during
  implementation that were left open in the design phase.

**Blocker before Stage 3.3 (Hydrology):** resolve cliff impassability
question (design doc §1.4) before implementing canyon walls. Not a
blocker for Stage 3.2 itself.

---

## Remaining pipeline stages

**Stage 3.3 — Hydrology** (design doc §1.11)  
Flow accumulation, sink resolution, lake detection, river identification,
floodplain marking, canyon carving. The climate moisture and
precipitation fields feed flow accumulation directly.

**Stage 3.4 — Biomes** (design doc §1.12)  
Whittaker-style (temperature, moisture) → biome lookup. Biome table as
RON config file. The first cross-stage RON config use.

**Stage 3.5 — Resource placement** (design doc §1.13)  
Geological placement using plate mineral character, biome filtering,
clustering for luxuries, scattered for strategics.

**Stage 3.6 — Starting positions** (design doc §1.14)  
Resource-aware viability scoring, hard adequacy floor, spacing
constraints. Final exercise of the determinism scaffold.