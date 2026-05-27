# Implementation Status

**Date:** 2026-05-27  
**Stage:** 3.5 Resource placement — **planning complete, implementation pending**  
**Next action:** Begin Stage 3.5 implementation

---

## Context

This project is a turn-based 2D strategy game inspired by Civilization,
built in Rust using the Bevy game engine. The codebase is a Cargo
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

All steps in `crates/worldgen/src/stages/tectonics/`. Plate seeds,
organic flood-fill growth, plate property assignment (type, motion
vector, mineral character), boundary classification, elevation field,
boundary-effect propagation, detail noise, sea-level threshold,
height-level discretisation, hills pass, mountain pass carving.

`MineralCharacter` is assigned per plate during this stage and is
available to Stage 3.5 via `TectonicData::plates`. The five variants
— `Ferrous`, `Precious`, `Gems`, `Hydrocarbons`, `Industrial` — are
defined in `world_data.rs` and weighted via `TectonicsParams::mineral_weights`.

### Stage 3.2 — Climate (complete)

All steps in `crates/worldgen/src/stages/climate/`. Conservative
donor-cell upwind advection, CFL-capped wind profile, three-phase
iteration (source → advection → precipitation), double-buffered fields,
fixed `(r, q)` iteration order for determinism.

### Stage 3.3 — Hydrology (complete)

Flow accumulation, sink resolution, lake detection (with stored outlet
coordinates for outflow), river identification, floodplain marking,
canyon carving. Lake outlets stored as `Field<Option<Coord>>` and used
during flow accumulation to prevent lakes from being pure sinks
(ADR-0013).

### Stage 3.4 — Biomes (complete)

Whittaker-style 2D lookup from (temperature, moisture) to biome,
with hydrology overrides and optional smoothing. Biome table stored as
a RON data asset at `assets/worldgen/biome_table.ron` (ADR-0016).
Forest density as a separate `Field<f32>` populated by low-frequency
noise, masked to forest biome tiles.

Six biomes implemented: `TropicalRainforest`, `TemperateForest`,
`BorealForest`, `Grassland`, `Desert`, `Tundra`.

### Gate: Stage 3.4 complete

- All 7 steps implemented; `BiomeData` populated by `generate_world`.
- `maptest run` and `mapdump` produce 2 new PNGs: categorical biome map
  and forest density field.
- Categorical colormap assigns a visually separable colour to each of
  the 6 biomes.
- Default table validates as full 8×8 coverage at pipeline startup.
- Hydrology overrides produce a visible fertile strip in at least one
  desert-containing seed at default `hydrology_override_strength`.
- Hydrology overrides produce a visible fertile strip in at least one
  desert-containing seed at default `hydrology_override_strength`.
- Determinism test extended to cover `biome` and `forest_density`
  fields; passes across multiple runs.
- ADR-0008 written and accepted.
- `documents/planning/implementation-plan.md` updated to mark Stage 3.4
  complete and Stage 3.5 next.
- Design doc §1.12 updated with decisions made during implementation.
- `documents/planning/add-biomes-pipeline.md` created.

---

## Stage 3.5 — Resource placement (next)

Design doc §1.13. ADR-0017 records all decisions made during planning.

### Decisions made (see ADR-0017)

- **One resource per tile.** `ResourceData` holds `Field<Option<ResourceId>>`.
- **Three placement behaviours by category:**
    - *Luxury:* ~5% baseline scatter + ~95% plate-mineral-character
      concentration, clustered into 2–4 tile groups.
    - *Strategic:* mixed baseline/concentration split per resource,
      ensuring no region is locked out of early-game necessities.
    - *Bonus:* biome/terrain filtered only, uniform scatter, no plate
      weighting.
- **First-pass resource list (10 resources):**
    - Strategic: iron, horses
    - Luxury: gold, gems, spices, wine
    - Bonus: wheat, fish, stone
- **Strategic baseline/concentration:** iron 60/40, horses 70/30.
  Later-tech strategics (coal, oil, uranium) deferred — same algorithm,
  content addition only.
- **Ocean tiles** (fish) use plate mineral character via the existing
  `PlateId` field. No special casing required.
- **Single RON config file** at `assets/worldgen/resources.ron`,
  structured into `strategic`, `luxury`, and `bonus` sections.

### Step roster

Steps are listed in dependency order; numbering is `3.5.N` to match
the mapdump prefix scheme (`35_NN_<name>.png`).

**Step 1 — `ResourceId` enum and `ResourceData` struct**

Define `ResourceId` as a fieldless enum with one variant per resource
(`Iron`, `Horses`, `Gold`, `Gems`, `Spices`, `Wine`, `Wheat`, `Fish`,
`Stone`). Add `ResourceData` holding `resource: Field<Option<ResourceId>>`.
Add `ResourceData` to `WorldData`; re-export from `lib.rs`. Pure data;
no generation logic yet.

**Step 2 — RON config and loader**

Define `ResourceConfig` as a named map of resource entries, each
carrying the per-resource placement parameters (see ADR-0017 §7 for
the full field list). Add a `validate` method that checks:

- All `ResourceId` variants have an entry.
- `baseline_fraction` in `[0.0, 1.0]` for strategic resources.
- `rarity` in `(0.0, 1.0]`.
- `min_spacing >= 1`.
- `cluster_radius >= 1` if `cluster = true`.
- Mineral affinity weights are non-negative.

Load-time failure via a new `GenerationError::InvalidResourceConfig`
variant. Write `assets/worldgen/resources.ron` with initial defaults
from the values in ADR-0017.

**Step 3 — Eligible tile filtering**

For each resource, build the set of eligible tiles from biome,
terrain, and (for water resources) tile type. Apply latitude preference
if configured. Store as a temporary `Vec<Coord>` per resource; this is
the input to both placement passes.

Filtering is deterministic given stable `WorldData`; no RNG consumed
here.

**Step 4 — Bonus resource placement**

Uniform Poisson-disk scatter across eligible tiles with minimum
spacing. Target count from `rarity × eligible_tile_count`. No plate
weighting. Writes directly into `ResourceData::resource`.

**Step 5 — Strategic resource placement (baseline pass)**

For each strategic resource, place `baseline_fraction × target_count`
deposits via Poisson-disk scatter across all eligible tiles, ignoring
plate mineral character. This is the floor that prevents any region
from being completely locked out.

**Step 6 — Strategic resource placement (concentration pass)**

Place the remaining `(1 − baseline_fraction) × target_count` deposits
using plate-weighted eligible tiles. Weight each tile by the mineral
affinity of its plate's `MineralCharacter`. Poisson-disk with the
same minimum spacing as the baseline pass. Tiles already occupied from
the baseline pass are excluded.

**Step 7 — Luxury resource placement**

For each luxury resource, place `baseline_fraction × target_count`
tiles via uniform scatter (baseline, same as strategic step 5), then
place the remainder using plate-weighted Poisson-disk seed placement
followed by neighbour expansion to 2–4 tile clusters. Cluster
expansion skips tiles already occupied or not eligible.

**Step 8 — Debug visualization and tuning integration**

Add per-resource PNG outputs to `output.rs` and `mapdump`: one binary
overlay per resource (resource present/absent), and one combined
resource overlay with a categorical colormap (one colour per
`ResourceId`). Add `ResourceParams` scalar fields to `set_param`
in `tools/maptest/src/sweep.rs`. Extend `tools/maptest/presets/default.ron`
with a `resources:` section.

**Step 9 — Determinism tests**

Extend the determinism test to cover the `resource` field. Update
reference hashes.

### File structure

Add under `crates/worldgen/src/stages/`:

```
stages/
└── resources.rs                      ← run() entry point
└── resources/
    ├── config.rs                     ← RON config types and loader
    ├── filter.rs                     ← step 3: eligible tile filtering
    ├── scatter.rs                    ← Poisson-disk scatter (shared)
    ├── bonus.rs                      ← step 4: bonus placement
    ├── strategic.rs                  ← steps 5–6: baseline + concentration
    └── luxury.rs                     ← step 7: clustered luxury placement
```

`world_data.rs` gains `ResourceId` and `ResourceData` alongside
`BiomeData`. `params.rs` gains `params/resources.rs` exporting
`ResourceParams` (scalar tuning fields; the resource config is a data
asset like the biome table). `lib.rs` adds the resources stage call
after biomes, threading `StageRng` with `StageId::Resources`.

### Parameters

Initial defaults to lock in during tuning. Hard bounds enforced by
`validate`.

| Parameter | Default | Hard bounds | Notes |
|---|---|---|---|
| `resource_config_path` | `assets/worldgen/resources.ron` | — | Path to the RON config. |
| `cluster_expansion_attempts` | 8 | 1–32 | Neighbour candidates tried per cluster seed. |
| `poisson_max_attempts` | 30 | 5–100 | Rejection-sampling attempts per candidate. |

Per-resource rarity, spacing, and affinity weights live in the RON
config, not in `ResourceParams`. They are content data, not generation
infrastructure.

### Tuning targets

- **No region locked out of iron.** At least one iron deposit within
  30 tiles of any land tile, across at least 20 seeds.
- **Luxury monopolies visible.** Gems and gold concentrate clearly on
  plates of matching mineral character; adjacent continental plates
  with different characters have different luxuries or none.
- **Bonus resources match terrain.** Wheat on grassland and floodplain;
  stone on hilly and rocky tiles; fish on coast and shallow ocean.
- **No over-density.** No resource covers more than 8% of eligible
  tiles at default rarity.
- **Clusters look natural.** Luxury clusters read as deposits, not
  grids or straight lines.

### Key sweeps

1. **Iron `baseline_fraction` `0.4..0.8` × 5 steps** — find the
   minimum baseline that guarantees regional coverage without
   flooding the map.
2. **Luxury `rarity` `0.005..0.03` × 5 steps** — calibrate scarcity
   so monopolies are meaningful but not invisible.
3. **`cluster_radius` `1..4` × 4 steps** — verify that clusters read
   as deposits and don't overlap.
4. **Combined resource overlay across 10 seeds** — visual audit that
   no seed produces a degenerate resource distribution.

### ADRs written

- **ADR-0017:** Resource placement model. Records all decisions listed
  above: one resource per tile, three placement behaviours by category,
  first-pass resource list, strategic baseline/concentration split,
  ocean tile handling, single RON config file structure.

### Gate: Stage 3.5 complete

- All 9 steps implemented; `ResourceData` populated by `generate_world`.
- `maptest run` and `mapdump` produce per-resource PNG overlays and a
  combined categorical resource map.
- Iron present within 30 tiles of any land tile across 20 seeds.
- Luxury monopolies visually apparent on plates of matching mineral
  character.
- Bonus resources match their terrain eligibility rules.
- No resource exceeds 8% of eligible tiles at default rarity.
- Determinism test extended to cover `resource` field; reference hash
  updated.
- ADR-0017 written and accepted.
- `documents/planning/implementation-plan.md` updated to mark Stage 3.5
  complete and Stage 3.6 next.
- Design doc §1.13 updated with decisions made during planning.

---

## Remaining pipeline stages

**Stage 3.6 — Starting positions** (design doc §1.14)  
Resource-aware viability scoring, hard adequacy floor, spacing
constraints. Final exercise of the determinism scaffold.