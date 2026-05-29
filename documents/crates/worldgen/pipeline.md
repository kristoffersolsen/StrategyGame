# worldgen pipeline

This document explains the current `worldgen` pipeline as implemented by
`generate_world` in `crates/worldgen/src/lib.rs`.

The pipeline is deterministic: the same `WorldParams` value produces the same
`WorldData` output. Stage-local randomness is derived from a master seed through
`MasterRng::fork(StageId)`, so changing one stage's parameters does not shift the
random stream used by another stage.

Starting positions are planned as Stage 3.6 but are not currently run by
`generate_world`.

## Overall Flow

```text
WorldParams
  seed
  BasicParams
  AdvancedParams
  TuningParams
  biome_table_path
  resource_config_path
        |
        v
validate params
        |
        v
create MapBounds from map size
        |
        v
load biome table + resource config
        |
        v
Stage 3.1 tectonics  -> TectonicData
Stage 3.2 climate    -> ClimateData
Stage 3.3 hydrology  -> HydrologyData
Stage 3.4 biomes     -> BiomeData
Stage 3.5 resources  -> ResourceData
        |
        v
WorldData
```

## Input Layers

`WorldParams` is split into three exposure layers:

| Layer | Struct | Role |
|-------|--------|------|
| Flavor | `BasicParams` | Map size and broad map archetype. |
| Strategic | `AdvancedParams` | Land fraction, climate band, world age, resource abundance, player count. |
| Tuning | `TuningParams` | Internal stage knobs, grouped by pipeline stage. |

Only the tuning layer exposes algorithm details. A non-default tuning field disables
achievements.

## Determinism Model

`generate_world` creates one `MasterRng` from `WorldParams::seed`.
Each random stage forks its own deterministic sub-RNG:

| Stage | Fork |
|-------|------|
| Tectonics | `StageId::Tectonics` |
| Climate | `StageId::Climate` |
| Biomes | `StageId::Biomes` |
| Resources | `StageId::Resources` |

Hydrology is currently deterministic without random input. The rule is that generation
must not use thread-local randomness, non-deterministic iteration order, or unordered
parallel reductions that affect output.

## Stage 3.1 - Tectonics

Inputs:

| Input | Use |
|-------|-----|
| `MapBounds` | Map dimensions and wrap-aware neighbor operations. |
| `TectonicsParams` | Plate counts, elevation knobs, level proportions, hills, passes. |
| `StageRng` | Plate placement, plate properties, noise seeds, pass noise seed. |

Outputs:

| Output field | Meaning |
|--------------|---------|
| `seeds` | Plate seed coordinates. |
| `plate_id` | Per-tile plate ownership. |
| `plates` | Per-plate type, motion, and mineral character. |
| `boundary_type` | Per-tile tectonic boundary classification. |
| `elevation` | Continuous elevation, normalized so sea level is zero. |
| `height_level` | Discrete ocean, coast, land, and mountain levels. |
| `is_hilly` | Per-tile hills feature. |

Internal steps:

| Step | Operation |
|------|-----------|
| 1 | Place plate seeds with Poisson-disk sampling. |
| 2 | Grow plate territories by organic flood fill. |
| 3 | Assign plate type, motion vector, mineral character, growth variance, and type clumping. |
| 4 | Classify boundaries as convergent, divergent, or transform. |
| 5 | Build base elevation from plate type and intra-plate noise. |
| 6 | Apply tectonic boundary elevation effects with Gaussian falloff. |
| 7 | Add multi-octave detail noise. |
| 8 | Quantize elevation by level proportions and normalize sea level. |
| 9 | Classify hills with boundary proximity and noise. |
| 10 | Carve mountain passes by demoting selected mountain tiles. |

Tectonics owns the base geography. Later stages read its elevation, height levels,
hills, plate IDs, and mineral characters; they do not rewrite the tectonic fields.
Hydrology carries its own post-hydrology `height_level` copy for debug and downstream
use.

## Stage 3.2 - Climate

Inputs:

| Input | Use |
|-------|-----|
| `MapBounds` | Latitude, wrap-aware iteration, and field dimensions. |
| `TectonicData` | Elevation, height levels, and water/land masks. |
| `ClimateParams` | Temperature, wind, evaporation, precipitation, diffusion, iteration count. |
| `StageRng` | Wind-noise seeds. |

Outputs:

| Output field | Meaning |
|--------------|---------|
| `temperature` | Normalized temperature field. |
| `moisture` | Steady-state moisture after advection. |
| `wind` | Per-tile `[east, south]` wind velocity. |
| `precipitation` | Accumulated precipitation over all iterations. |

Internal steps:

| Step | Operation |
|------|-----------|
| 1 | Build latitude-based temperature and apply elevation lapse rate. |
| 2 | Build zonal wind field and add low-frequency noise. |
| 3 | Build water evaporation and coastal moisture source fields. |
| 4 | Advect moisture with donor-cell upwind transport. |
| 5 | Remove moisture through base, temperature-capacity, and orographic precipitation. |
| 6 | Repeat advection and precipitation for a fixed iteration count. |
| 7 | Return final moisture and accumulated precipitation. |

The wind parameters are validated against the CFL cap. This is load-bearing for
numerical stability because each iteration moves a fraction of tile moisture across
hex edges.

## Stage 3.3 - Hydrology

Inputs:

| Input | Use |
|-------|-----|
| `MapBounds` | Drainage neighbors and deterministic traversal. |
| `TectonicData` | Continuous elevation and initial height levels. |
| `ClimateData` | Precipitation used as flow source. |
| `HydrologyParams` | Lake threshold, river thresholds, floodplain tolerance, canyon carving. |

Outputs:

| Output field | Meaning |
|--------------|---------|
| `flow_direction` | Single downstream D6 direction per land tile, or terminal. |
| `flow_accumulation` | Normalized precipitation-weighted accumulated flow. |
| `is_lake` | Inland lake tiles. |
| `lake_outlet` | Spill point for lake outflow. |
| `river_size` | Creek, river, or major river classification. |
| `is_floodplain` | Low adjacent tiles near rivers. |
| `is_canyon` | River tiles lowered by canyon carving. |
| `is_canyon_wall` | Flanking canyon-wall tiles. |
| `height_level` | Post-hydrology height-level copy. |

Internal steps:

| Step | Operation |
|------|-----------|
| 1 | Build initial flow direction by steepest descent. |
| 2 | Detect land basins ending in sink tiles. |
| 3 | Resolve basins by small-basin filling or lake marking. |
| 4 | Rebuild flow direction after basin resolution. |
| 5 | Accumulate precipitation downstream, including lake outlet propagation. |
| 6 | Classify creeks, rivers, and major rivers from normalized flow. |
| 7 | Mark floodplains adjacent to rivers and close in elevation. |
| 8 | Carve canyons where `flow * slope` is high enough. |

Flow direction uses descending elevation order. Ties at the same downhill elevation
prefer already-active channels, then the fixed hydrology direction order. This keeps
drainage networks consolidated and deterministic.

## Stage 3.4 - Biomes

Inputs:

| Input | Use |
|-------|-----|
| `MapBounds` | Field dimensions and neighbor checks. |
| `ClimateData` | Temperature and moisture. |
| `HydrologyData` | Land/lake/mountain eligibility and hydrology overrides. |
| `BiomeParams` | Table dimensions, override strength, smoothing, forest noise. |
| `BiomeTable` | Designer-authored biome lookup table and hydrology override map. |
| `StageRng` | Forest-density noise seed. |

Outputs:

| Output field | Meaning |
|--------------|---------|
| `biome` | Optional land biome. Ocean, coast, lake, and mountain tiles are `None`. |
| `forest_density` | Rendering hint for forest biomes, in `[0.0, 1.0]`. |

Internal steps:

| Step | Operation |
|------|-----------|
| 1 | Load and validate the biome table before pipeline execution. |
| 2 | Compute climate min/max ranges over biome-eligible land. |
| 3 | Bin temperature and moisture and assign base biomes by table lookup. |
| 4 | Apply hydrology overrides for rivers, floodplains, and lake-adjacent tiles. |
| 5 | Optionally smooth isolated one-tile biome artifacts. |
| 6 | Generate forest-density noise masked to forest biomes. |

Biome assignment uses current map-relative temperature and moisture ranges, not fixed
absolute climate thresholds. This lets the table remain useful across unusual climate
parameter sets while preserving deterministic behavior.

## Stage 3.5 - Resources

Inputs:

| Input | Use |
|-------|-----|
| `TectonicData` | Plate mineral character, plate ID, hills. |
| `HydrologyData` | Water, lakes, post-hydrology height levels. |
| `BiomeData` | Land biome eligibility. |
| `ResourceConfig` | Designer-authored per-resource placement data. |
| `ResourceParams` | Scatter attempt limit and cluster expansion attempts. |
| `ResourceAbundance` | Advanced-layer abundance multiplier. |
| `StageRng` | Resource placement randomness. |

Outputs:

| Output field | Meaning |
|--------------|---------|
| `resource` | Optional `ResourceId`; at most one resource per tile. |

Internal steps:

| Step | Operation |
|------|-----------|
| 1 | Load and validate the resource config before pipeline execution. |
| 2 | Build eligible tile lists from biome, terrain, hydrology, and latitude filters. |
| 3 | Place bonus resources by uniform Poisson-style scatter. |
| 4 | Place strategic baseline deposits uniformly. |
| 5 | Place strategic concentration deposits by mineral-character weighting. |
| 6 | Place luxury baseline deposits uniformly. |
| 7 | Place luxury concentration deposits as small clustered deposits. |

The placement target for each resource is:

```text
round(eligible_tile_count * rarity * abundance_multiplier)
```

The target is capped at the eligible tile count. Actual placement can be lower if
spacing, occupancy, or attempt limits prevent all deposits from being placed.

