# Implementation Status

**Date:** 2026-05-27  
**Stage:** 3.4 Biomes — **complete**  
**Next action:** Stage 3.5 — Resource placement (design §1.13)

---

## Context

This project is a turn-based 2D strategy game inspired by Civilization,
built in Rust using the Bevy game engine. The codebase is in a Cargo
workspace with four library/game crates (`coords`, `worldgen`, `game`,
`det_rng`) and two CLI tools (`mapdump`, `maptest`). This document
covers the state of the `worldgen` crate and the generation pipeline
only.

Stages 0–3.4 are complete. Stage 3.5 (resource placement) is next.

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

(Stage 3.1 summary, file structure, parameters table, and default
values are unchanged from the prior version of this document. The plan
below assumes that work is in place.)

### Stage 3.2 — Climate (complete)

All 9 steps in `crates/worldgen/src/stages/climate/`. Key decisions
carried forward:

- Conservative donor-cell upwind discretisation of the advection PDE.
  Wind projections computed in Cartesian hex space; six neighbour unit
  vectors spaced 60° apart starting from east.
- CFL hard cap `|v| ≤ 0.25` per tile per iteration. Validated at
  parameter load time.
- Smooth wind profile `u(φ) = −A·cos(3π·φ)` plus low-frequency noise
  perturbation.
- Three-phase iteration: source → advection → precipitation, with
  double-buffered fields.
- Fixed `(r, q)` iteration order for floating-point determinism.
- East-west wrap natural via `MapBounds::normalize`; north-south is a
  hard boundary (moisture leaving polar rows is lost).
- All water tiles are moisture sources, not just `ocean`.

### Gate: Stage 3.2 complete

- All 9 steps implemented; `ClimateData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 7 new PNGs across the four
  output categories.
- Visible rain shadows behind mountain chains across a range of seeds.
- Latitude bands clearly visible in the precipitation map: wet tropics,
  dry subtropics, wet mid-latitudes, dry poles.
- No horizontal stripe artefacts at default `wind_noise_amplitude`.
- CFL validation catches a deliberately-bad `wind_amplitude = 0.3`
  parameter at load time.
- Determinism test extended to cover all four climate fields; passes
  across multiple runs.
- ADR-0006 (climate iteration count and determinism strategy) and
  ADR-0007 (hex grid advection discretisation) written and accepted.
- `documents/planning/implementation-plan.md` updated to mark Stage 3.2
  complete and Stage 3.3 next.
- Design doc §1.10 updated with decisions made during implementation.

### Stage 3.3 — Hydrology (complete)

All steps in `crates/worldgen/src/stages/hydrology/`. Key decisions
carried forward:

- Hybrid sink resolution: basins below the area threshold are filled
  (Planchon-Darboux style); basins above the threshold become lakes with
  flow routed through the lowest spill point.
- Flow accumulation via topological sort by elevation (highest first);
  one pass, O(tiles).
- River identification: tiles where accumulated flow exceeds the
  configured threshold. Flow value retained for rendering tier
  (creek / river / major river).
- Floodplains: tiles adjacent to high-flow rivers within a configured
  elevation tolerance.
- Canyon carving: post-hoc pass over river tiles where `flow × slope`
  exceeds the carving threshold. Canyon tiles lowered 1–2 levels;
  perpendicular neighbours marked as canyon walls.
- Cliff impassability question (design doc §1.4) resolved: canyon walls
  are impassable. Recorded in ADR.
- Fixed tie-breaking throughout sink resolution for determinism.

### Gate: Stage 3.3 complete

- All steps implemented; `HydrologyData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 5 new PNGs: flow accumulation
  (log scale), rivers overlay, lakes overlay, floodplains overlay,
  canyons overlay.
- Every flow path terminates at sea or a designated lake across a range
  of seeds — no orphan flow.
- Lake counts are plausible; no pit-lake explosion at default
  parameters.
- Canyon carving produces visible chokepoints without overpopulating
  the map.
- Determinism tests extended to cover all hydrology fields; pass across
  multiple runs.
- ADR for cliff/canyon-wall traversability written and accepted.
- `documents/planning/implementation-plan.md` updated to mark Stage 3.3
  complete and Stage 3.4 next.
- Design doc §1.11 updated with decisions made during implementation.

---

### Stage 3.4 — Biomes (complete)

Biome assignment is a readout stage: every land tile is classified using
the temperature and moisture fields from Stage 3.2, with overrides
applied from Stage 3.3's hydrology data. The hard simulation work is
complete; this stage is primarily a lookup table plus a small number of
override and smoothing passes.

The main technical challenge is the biome table structure itself — it
ships as a committed RON data file, the first cross-stage RON data asset
(as opposed to a params struct). The `validate()` call at pipeline
startup must reject any table that leaves a reachable `(temperature_bin,
moisture_bin)` combination unassigned, making table coverage a hard
load-time invariant rather than a runtime surprise.

See `documents/pipelines/add_biomes_pipeline.md` for how to extend the
biome set.

### Key decisions carried into implementation

- **Whittaker 2D lookup.** Temperature and moisture are kept continuous
  until the lookup step. The table maps `(temperature_bin, moisture_bin)
  → Biome`. Bins are closed intervals covering the full range produced
  by Stage 3.2; any unhandled pair is a hard error at table load time
  via `GenerationError::InvalidBiomeTable`.
- **Biome set (land tiles only):** `TropicalRainforest`,
  `TemperateForest`, `BorealForest`, `Grassland`, `Desert`, `Tundra`.
  Ocean, coast, and lake tiles are not biome-assigned; they are handled
  by terrain type from earlier stages.
- **Hydrology overrides (applied after lookup):** river tiles, floodplain
  tiles, and lake-adjacent tiles shift toward wetter/fertile biome
  variants. Override strength is a scalar param; `1.0` means full
  reclassification, `0.0` disables overrides entirely. Default: `0.8`.
- **High-elevation treatment:** mountain-level tiles (already impassable
  terrain) receive no biome assignment. The tile one level below
  (sub-alpine) is left to the Whittaker lookup; the result is acceptable
  without a special case if it reads as `BorealForest` or `Tundra`,
  which it typically will at that temperature range. Revisit during
  tuning if the visual result is wrong.
- **Smoothing pass (optional, off by default):** single-tile biome
  islands can flip to the majority biome of their six land neighbours.
  `smoothing_passes = 0` disables entirely. Double-buffered; fixed
  `(r, q)` scan order for determinism.
- **Forest density as a feature, not a biome:** `BiomeData` stores the
  biome classification only. A separate `forest_density: Field<f32>`
  field in `[0.0, 1.0]` is populated by a low-frequency noise pass
  using the biome as a mask. Non-forest tiles receive `0.0`. This field
  is a rendering hint; it is not consumed by resources or starting
  positions.
- **Lookup table file location:** `assets/worldgen/biome_table.ron`.
  Read at pipeline startup; path configurable via `WorldParams`. The
  committed file contains standard Whittaker defaults (8×8 grid). The
  bin counts in the file must match `BiomeParams::temperature_bin_count`
  and `BiomeParams::moisture_bin_count` or `validate()` rejects the
  table.
- **`BTreeMap<Biome, Biome>` for hydrology overrides** in `BiomeTable`.
  `Biome` derives `Ord` so iteration order is deterministic without
  sorting. `HashMap` is not used; see the determinism rules in
  `crates/worldgen/src/lib.rs`.

### Gate: Stage 3.4 complete

- All 7 steps implemented; `BiomeData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 2 new PNGs: categorical biome map
  and forest density field.
- Categorical colormap assigns a visually separable color to each of
  the 6 biomes.
- Default table validates as full 8×8 coverage at pipeline startup.
- Hydrology overrides are wired for river, floodplain, and lake-adjacent
  tiles with default `hydrology_override_strength = 0.8`.
- Determinism test extended to cover `biome` and `forest_density`
  fields; reference hash updated.
- ADR-0016 written and accepted.
- `documents/pipelines/add_biomes_pipeline.md` exists.

## What is next

**Stage 3.5 — Resource placement** (design doc §1.13)

Geological placement using plate mineral character, biome filtering,
clustering for luxuries, and scattered placement for strategics.

<!-- Historical Stage 3.4 implementation roster follows for reference. -->

### Step roster

The stage breaks into seven steps. Steps are listed in dependency order;
numbering is `3.4.N` to match the mapdump prefix scheme
(`34_NN_<name>.png`).

1. **`Biome` enum and `BiomeData` struct** — define the `Biome` enum
   (`TropicalRainforest` … `Tundra`), the `BiomeTable`
   RON-deserializable struct, and `BiomeData` holding
   `biome: Field<Option<Biome>>` and `forest_density: Field<f32>`.
   Add `BiomeData` to `WorldData`; re-export from `lib.rs`. Pure data;
   no generation logic yet.

2. **Biome table RON file and loader** — define `BiomeTable` as a list
   of `(t, m, biome)` entries plus a `BTreeMap<Biome, Biome>` for
   hydrology overrides. Add a `validate` method that checks full grid
   coverage for the configured bin counts. Load-time failure via
   `GenerationError::InvalidBiomeTable`. Write
   `assets/worldgen/biome_table.ron` with initial Whittaker defaults
   (8×8 grid; reference copy already drafted). Add `biome_table_path`
   to `WorldParams`.

3. **Base biome lookup** — for each land tile, bin the continuous
   temperature and moisture values and look up the biome. Ocean, coast,
   and lake tiles are left as `None`. Deterministic; no RNG.

4. **Hydrology overrides** — single pass over river tiles, floodplain
   tiles, and lake-adjacent tiles. For each, check the current biome
   against the override map and reclassify, scaled by
   `hydrology_override_strength`. Deterministic; no RNG.

5. **Optional smoothing pass** — `smoothing_passes` iterations of
   single-tile-island flip. Double-buffered; fixed `(r, q)` scan order.
   At `smoothing_passes = 0`, skip entirely.

6. **Forest density feature pass** — low-frequency noise keyed to the
   stage RNG, masked to forest biome tiles. Non-forest tiles get `0.0`.

7. **Biome stage entry point, determinism tests, and tuning
   integration** — wire steps 1–6 in `stages/biomes.rs`. Thread
   `StageRng` with `StageId::Biomes`. Add biome map (categorical
   colormap) and forest density (Viridis) outputs to `output.rs` and
   `mapdump`. Extend the determinism test to cover `biome` and
   `forest_density` fields.

### File structure

Add under `crates/worldgen/src/stages/`:

```
stages/
└── biomes.rs                        ← run() entry point
└── biomes/
    ├── lookup.rs                    ← step 3: base biome assignment
    ├── hydrology_overrides.rs       ← step 4: river/floodplain/lake-adj overrides
    ├── smoothing.rs                 ← step 5: single-tile-island smoothing
    └── forest_density.rs            ← step 6: forest density noise pass
```

`world_data.rs` gains a `BiomeData` sub-struct alongside `ClimateData`
and `HydrologyData`, carrying `biome` and `forest_density` fields plus
the `empty(bounds)` constructor.

`params.rs` gains a `params/biomes.rs` module exporting `BiomeParams`,
added as `TuningParams::biomes`. Scalar fields rustdoc'd individually,
`serde` derived, `validate` enforces range constraints and checks that
bin counts agree with the loaded table.

`lib.rs` adds the biomes stage call after hydrology, threading
`StageRng` with `StageId::Biomes`.

The biome table is a data asset, not a params struct. Its path lives in
`WorldParams`; the default points to the committed asset. See ADR-0016.

### Parameters

Initial defaults to lock in during tuning. Hard bounds enforced by
`validate`; recommended ranges enforced by the maptest preset defaults.

| Parameter | Default | Hard bounds | Notes |
|---|---|---|---|
| `temperature_bin_count` | 8 | 2–32 | Grid rows; must match table file. |
| `moisture_bin_count` | 8 | 2–32 | Grid columns; must match table file. |
| `hydrology_override_strength` | 0.8 | 0.0–1.0 | `1.0` = full reclassification. |
| `smoothing_passes` | 0 | 0–5 | Iterations of single-tile-island smoothing. |
| `forest_noise_frequency` | 0.4 | 0.05–2.0 | Spatial frequency (map-width units). |
| `forest_noise_amplitude` | 0.6 | 0.0–1.0 | Amplitude of density variation around `0.5`. |

**Note:** `temperature_bin_count` and `moisture_bin_count` are not
freely sweepable in isolation. Changing either requires regenerating
the table file to match, otherwise `validate()` fails at load time.
Document this constraint in the param rustdoc and the maptest preset
comment.

### Tuning targets

- **Biome transitions look natural.** No hard stripes at bin boundaries.
  Continuous input fields should produce smooth-looking transitions even
  with a coarse 8×8 grid.
- **No unhandled climate combinations.** `validate()` must pass for the
  default table across at least 20 seeds. If Stage 3.2 tuning has pushed
  temperature or moisture outside the table's current coverage, extend
  the table before proceeding.
- **Hydrology overrides produce a visible fertile strip.** In seeds with
  desert biomes adjacent to rivers, the override must be visually
  obvious in the categorical map at default `hydrology_override_strength
  = 0.8`. The Nile pattern is the benchmark.
- **Forest density is spatially coherent.** The density field should
  show continent-scale variation, not noise-only scatter.
- **Smoothing is conservative.** At `smoothing_passes = 1`, isolated
  single-tile anomalies disappear without smearing legitimate biome
  edges. Verify on seeds with mountain-adjacent biome transitions.

### Key sweeps

1. **`hydrology_override_strength` `0.0..1.0` × 5 steps** — confirms
   override is visually effective and identifies the right default.
2. **`smoothing_passes` `0..3` × 4 steps** — verify legitimate boundary
   preservation; confirm noise removal at 1 pass.
3. **Bin counts spot-check at `4`, `8`, `16`** — verify that `8×8` is
   not so coarse as to produce blocky biome zones. Not a sweep; run
   manually with three table files.
4. **`forest_noise_frequency` `0.1..0.8` × 5 steps** — find the
   frequency that produces continent-scale coherence.

### Tuning integration

- Add `BiomeParams` match arms to `tools/maptest/src/sweep.rs` →
  `set_param` (scalar fields only; bin counts noted as non-sweepable
  in comments).
- Add biome map (categorical colormap) and forest density (Viridis)
  image-writing calls to `tools/maptest/src/output.rs` →
  `write_world_outputs`.
- Extend `tools/maptest/presets/default.ron` to include `biomes:` with
  every field at its default and per-field comments.
- Update `tools/maptest/README.md` sweepable-fields list and
  output-files table.

### ADRs to write

- **ADR-0008 (proposed): Biome table as a data asset, not a params
  struct.** Records the decision to store the biome lookup table as a
  committed RON data file (`assets/worldgen/biome_table.ron`) rather
  than embedding it in `BiomeParams`. Rationale: the table has dozens
  of entries and is designer-editable — embedding it in a params struct
  would break the `set_param` scalar-sweep model and make the preset
  files unreadable. Alternatives considered: hardcoded `match` in Rust
  (rejected: not designer-editable), embedded as a flat vec in
  `BiomeParams` (rejected: unreadable, un-diffable, hard to extend).
  Cross-references ADR-0003 (RON infrastructure). Notes the bin-count
  coupling constraint and the `validate()` enforcement mechanism.

### Gate: Stage 3.4 complete

- All 7 steps implemented; `BiomeData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 2 new PNGs: categorical biome map
  and forest density field.
- Categorical colormap assigns a visually separable colour to each of
  the 6 biomes. No two adjacent biomes are ambiguous at a glance.
- Default table covers the full temperature × moisture range produced
  by Stage 3.2 across at least 20 seeds; `validate()` passes on all.
- Hydrology overrides produce a visible fertile strip in at least one
  desert-containing seed at default `hydrology_override_strength`.
- Determinism test extended to cover `biome` and `forest_density`
  fields; passes across multiple runs.
- ADR-0008 written and accepted.
- `documents/planning/implementation-plan.md` updated to mark Stage 3.4
  complete and Stage 3.5 next.
- Design doc §1.12 updated with decisions made during implementation.
- `documents/planning/add-biomes-pipeline.md` created (drafted
  alongside the Stage 3.4 plan; no additional work required at gate).

---

## Remaining pipeline stages

**Stage 3.5 — Resource placement** (design doc §1.13)  
Geological placement using plate mineral character, biome filtering,
clustering for luxuries, scattered for strategics.

**Stage 3.6 — Starting positions** (design doc §1.14)  
Resource-aware viability scoring, hard adequacy floor, spacing
constraints. Final exercise of the determinism scaffold.
