# Biome Stage - Parameter Reference

Parameters for Stage 3.4. Consumed by `BiomeParams` in
`crates/worldgen/src/params/biomes.rs`.

The biome stage assigns land biomes from climate fields through a designer-authored
lookup table, applies hydrology-driven fertile overrides, optionally smooths isolated
artifacts, and emits a forest-density rendering hint.

Two sets of defaults exist:

| Parameter | Struct default | Preset default |
|-----------|----------------|----------------|
| `temperature_bin_count` | `8` | `8` |
| `moisture_bin_count` | `8` | `8` |
| `hydrology_override_strength` | `0.8` | `0.8` |
| `smoothing_passes` | `0` | `0` |
| `forest_noise_frequency` | `0.4` | `0.4` |
| `forest_noise_amplitude` | `0.6` | `0.6` |

The biome table itself is documented in [../data-assets.md](../data-assets.md).

## Steps 2-3 - Climate Binning And Table Lookup

### `temperature_bin_count`

| | |
|---|---|
| Type | `u8` |
| Struct default | `8` |
| Preset default | `8` |
| Range | `2..=32` |

Number of temperature bins expected by the biome table.

This is not a free scalar tuning knob. Changing it requires a biome table whose
`temperature_bin_count` matches the new value exactly. The pipeline validates the
table before generation.

Temperature is binned over the generated map's biome-eligible land range. Ocean,
coast, lake, and mountain tiles are excluded from the min/max range used for binning.

**Tuning:** Increase only when the biome table needs finer climate resolution.
Lowering simplifies the table but removes climate distinctions.

### `moisture_bin_count`

| | |
|---|---|
| Type | `u8` |
| Struct default | `8` |
| Preset default | `8` |
| Range | `2..=32` |

Number of moisture bins expected by the biome table.

Like `temperature_bin_count`, this must match the loaded biome table exactly.
Moisture is binned over the generated map's biome-eligible land range.

**Tuning:** Increase only when the biome table needs more moisture bands. The
asset grows as `temperature_bin_count * moisture_bin_count`, so high counts quickly
become expensive to maintain by hand.

## Step 4 - Hydrology Overrides

### `hydrology_override_strength`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.8` |
| Preset default | `0.8` |
| Constraint | finite, in `[0.0, 1.0]` |

Strength of hydrology-driven biome reclassification.

The biome table defines the actual replacement map, currently:

| Base biome | Hydrology replacement |
|------------|-----------------------|
| `Desert` | `Grassland` |
| `Grassland` | `TemperateForest` |

Hydrology influence is derived from the tile's water features:

| Feature | Influence |
|---------|-----------|
| Floodplain | `1.0` |
| Lake-adjacent | `1.0` |
| Major river | `1.0` |
| River | `0.8` |
| Creek | `0.5` |

The override is applied when:

```text
hydrology_override_strength >= 1.0 - influence
```

At the default `0.8`, major rivers, normal rivers, floodplains, and lake-adjacent
tiles can override, while creeks do not.

**Tuning:** `0.0` disables fertile overrides. `1.0` applies every eligible override,
including creeks. Values between those endpoints select stronger water influences
first.

## Step 5 - Conservative Smoothing

### `smoothing_passes`

| | |
|---|---|
| Type | `u8` |
| Struct default | `0` |
| Preset default | `0` |
| Range | `0..=5` |

Number of conservative smoothing passes for isolated one-tile biome artifacts.

A tile is considered for replacement only when none of its valid biome neighbors
share its current biome. It is replaced only if another biome appears in at least
three neighbors. Lake and non-biome neighbors are ignored.

**Tuning:** `0` preserves raw lookup output. `1` removes obvious isolated speckles.
Higher values increasingly homogenize biome edges and should be used carefully.

## Step 6 - Forest Density

### `forest_noise_frequency`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.4` |
| Preset default | `0.4` |
| Range | `[0.05, 2.0]` |

Spatial frequency of the wrap-aware OpenSimplex noise used for forest density.

Forest density is generated only for forest biomes:

| Forest biome |
|--------------|
| `TropicalRainforest` |
| `TemperateForest` |
| `BorealForest` |

All non-forest and non-biome tiles receive `0.0`.

**Tuning:** Lower values produce broad forest-density regions. Higher values produce
finer patches.

### `forest_noise_amplitude`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.6` |
| Preset default | `0.6` |
| Range | `[0.0, 1.0]` |

Amplitude of forest-density variation around `0.5`.

The generated density is:

```text
clamp(0.5 + noise * forest_noise_amplitude * 0.5, 0.0, 1.0)
```

**Tuning:** `0.0` makes every forest tile density `0.5`. Higher values increase
visual contrast between sparse and dense forest regions.

