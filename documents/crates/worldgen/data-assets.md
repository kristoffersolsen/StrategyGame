# worldgen data assets

`worldgen` loads two designer-editable RON assets before running the generation
pipeline:

| Asset | Used by | Default path field |
|-------|---------|--------------------|
| `assets/worldgen/biome_table.ron` | Stage 3.4 biomes | `WorldParams::biome_table_path` |
| `assets/worldgen/resources.ron` | Stage 3.5 resources | `WorldParams::resource_config_path` |

Relative paths are resolved from the current working directory first. If that fails,
the loader also tries the workspace root derived from the `worldgen` crate manifest.

## Biome Table

Source: `assets/worldgen/biome_table.ron`

Rust loader: `crates/worldgen/src/stages/biomes.rs::BiomeTable`

Purpose: map binned climate values to land biomes, then define hydrology-driven
override mappings.

Schema:

```ron
(
    temperature_bin_count: 8,
    moisture_bin_count: 8,
    entries: [
        ( t: 0, m: 0, biome: Tundra ),
        // one entry for every temperature/moisture bin pair
    ],
    hydrology_overrides: {
        Desert: Grassland,
        Grassland: TemperateForest,
    },
)
```

Validation:

| Rule | Error variant |
|------|---------------|
| File cannot be read | `GenerationError::InvalidBiomeTable` |
| RON cannot be parsed | `GenerationError::InvalidBiomeTable` |
| `temperature_bin_count` differs from `BiomeParams` | `GenerationError::InvalidBiomeTable` |
| `moisture_bin_count` differs from `BiomeParams` | `GenerationError::InvalidBiomeTable` |
| Any entry is outside the configured grid | `GenerationError::InvalidBiomeTable` |
| Any `(temperature_bin, moisture_bin)` pair is duplicated | `GenerationError::InvalidBiomeTable` |
| Any grid cell is missing | `GenerationError::InvalidBiomeTable` |

`entries` must cover the full rectangular grid:

```text
temperature_bin_count * moisture_bin_count
```

entries total.

Biome assignment bins the generated map's own biome-eligible land range. Bin `0`
is the coldest or driest generated eligible value, and the last bin is the hottest
or wettest generated eligible value. The comments in the default RON file describe
the intended calibration range, but runtime binning is map-relative.

Hydrology overrides are applied after base table lookup. The RON map controls which
biomes can be upgraded; `BiomeParams::hydrology_override_strength` controls which
water influences are strong enough to trigger the upgrade.

Valid biome names:

| Biome |
|-------|
| `TropicalRainforest` |
| `TemperateForest` |
| `BorealForest` |
| `Grassland` |
| `Desert` |
| `Tundra` |

## Resource Config

Source: `assets/worldgen/resources.ron`

Rust loader: `crates/worldgen/src/stages/resources/config.rs::ResourceConfig`

Purpose: define resource categories, eligible terrain and biomes, rarity, spacing,
cluster behavior, latitude preferences, and mineral-character affinities.

Top-level schema:

```ron
(
    strategic: {
        Iron: (
            category: Strategic,
            eligible_biomes: [],
            eligible_terrain: [FlatLand, Hills],
            latitude_preference: None,
            rarity: 0.018,
            cluster: false,
            cluster_radius: 1,
            min_spacing: 4,
            baseline_fraction: 0.60,
            mineral_affinities: {
                Ferrous: 1.0,
            },
        ),
    },
    luxury: {},
    bonus: {},
)
```

Every `ResourceId` must appear exactly once across `strategic`, `luxury`, and
`bonus`.

Valid resource IDs:

| Resource | Default category |
|----------|------------------|
| `Iron` | Strategic |
| `Horses` | Strategic |
| `Gold` | Luxury |
| `Gems` | Luxury |
| `Spices` | Luxury |
| `Wine` | Luxury |
| `Wheat` | Bonus |
| `Fish` | Bonus |
| `Stone` | Bonus |

### Resource Entry Fields

| Field | Type | Meaning |
|-------|------|---------|
| `category` | `Strategic`, `Luxury`, or `Bonus` | Must match the top-level section. |
| `eligible_biomes` | `Vec<Biome>` | Empty means any biome-assigned land tile, unless terrain targets water. |
| `eligible_terrain` | `Vec<TerrainFilter>` | OR-combined terrain filters. Empty means no terrain restriction. |
| `latitude_preference` | `Option<LatitudePreference>` | Optional normalized absolute latitude band. |
| `rarity` | `f32` | Target fraction of eligible tiles before abundance multiplier. |
| `cluster` | `bool` | Whether the entry is allowed to use cluster expansion. |
| `cluster_radius` | `u8` | Radius used for cluster expansion. |
| `min_spacing` | `u8` | Minimum distance between deposits of the same resource. |
| `baseline_fraction` | `f32` | Fraction placed by uniform baseline pass for strategic and luxury resources. |
| `mineral_affinities` | `BTreeMap<MineralCharacter, f32>` | Plate mineral weights for weighted placement. |

### Validation Rules

| Rule | Error variant |
|------|---------------|
| File cannot be read | `GenerationError::InvalidResourceConfig` |
| RON cannot be parsed | `GenerationError::InvalidResourceConfig` |
| A resource appears more than once | `GenerationError::InvalidResourceConfig` |
| A `ResourceId` is missing | `GenerationError::InvalidResourceConfig` |
| Entry `category` does not match its section | `GenerationError::InvalidResourceConfig` |
| `rarity` is not finite or not in `(0.0, 1.0]` | `GenerationError::InvalidResourceConfig` |
| `min_spacing == 0` | `GenerationError::InvalidResourceConfig` |
| `cluster == true` and `cluster_radius == 0` | `GenerationError::InvalidResourceConfig` |
| Strategic/luxury `baseline_fraction` is not finite or not in `[0.0, 1.0]` | `GenerationError::InvalidResourceConfig` |
| Any mineral affinity is not finite or is negative | `GenerationError::InvalidResourceConfig` |
| Latitude preference is non-finite, unordered, or outside `[0.0, 1.0]` | `GenerationError::InvalidResourceConfig` |

### Terrain Filters

| Filter | Matches |
|--------|---------|
| `Land` | Any non-water, non-mountain biome tile. |
| `Water` | Ocean, coast, or lake. |
| `Ocean` | Height level `0`, excluding lakes. |
| `Coast` | Height level `1`, excluding lakes. |
| `Lake` | Inland lake tile. |
| `FlatLand` | Biome tile where `tectonics.is_hilly == false`. |
| `Hills` | Biome tile where `tectonics.is_hilly == true`. |
| `Mountain` | Non-biome land tile above coast level. |

Terrain filters are OR-combined. Biome filters and latitude filters are then applied
as additional AND conditions.

### Latitude Preference

`latitude_preference` uses normalized absolute latitude:

```text
0.0 = equator
1.0 = poles
```

Schema:

```ron
Some((min: 0.20, max: 0.70))
```

### Mineral Affinities

Strategic and luxury resources use mineral affinities during weighted placement.
The tile's `plate_id` selects a plate, and the plate's `mineral_character` selects
the configured affinity weight. Missing affinities count as `0.0`. If the total
weight across all eligible tiles is `0.0`, the picker falls back to uniform choice.

Valid mineral characters:

| Mineral character |
|-------------------|
| `Ferrous` |
| `Precious` |
| `Gems` |
| `Hydrocarbons` |
| `Industrial` |

### Placement Categories

| Category | Placement behavior |
|----------|--------------------|
| Bonus | Uniform scatter only. |
| Strategic | Uniform baseline plus mineral-weighted concentration pass. |
| Luxury | Small uniform baseline plus mineral-weighted clustered deposits. |

All resources share the one-resource-per-tile output field. If a tile is already
occupied by an earlier category or resource, later placement attempts reject it.

