# worldgen parameter reference

This directory documents `TuningParams`, the full-tunability layer exposed by
`WorldParams`.

| Stage | Struct | Document |
|-------|--------|----------|
| 3.1 Tectonics | `TectonicsParams` | [tectonics.md](tectonics.md) |
| 3.2 Climate | `ClimateParams` | [climate.md](climate.md) |
| 3.3 Hydrology | `HydrologyParams` | [hydrology.md](hydrology.md) |
| 3.4 Biomes | `BiomeParams` | [biomes.md](biomes.md) |
| 3.5 Resources | `ResourceParams` | [resources.md](resources.md) |

## Defaults

Two kinds of defaults appear in tuning work:

| Default type | Location | Meaning |
|--------------|----------|---------|
| Struct defaults | `crates/worldgen/src/params/` | Determinism baseline used by `WorldParams::defaults()`. |
| Preset defaults | `tools/maptest/presets/default.ron` | Tuned working values used by map tools during development. |

When these differ, use the preset value for practical map-generation runs and the
struct value when documenting the Rust default contract.

## Achievement Gate

Any non-default field in `TuningParams` disables achievements. Basic and Advanced
parameters are player-facing controls; Tuning parameters are considered internal
knobs for experiments, testing, and map-authoring work.

## Asset-backed Parameters

Some stage behavior is configured by RON assets rather than Rust tuning fields:

| Asset | Document |
|-------|----------|
| `assets/worldgen/biome_table.ron` | [../data-assets.md](../data-assets.md) |
| `assets/worldgen/resources.ron` | [../data-assets.md](../data-assets.md) |

