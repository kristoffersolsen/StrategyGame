# worldgen crate documentation

`worldgen` is the deterministic procedural map-generation crate. It has no
Bevy dependency and is safe to run from tools, tests, and the game startup path.

The crate turns a `WorldParams` value into a complete `WorldData` map:

```text
WorldParams + seed
    -> tectonics
    -> climate
    -> hydrology
    -> biomes
    -> resources
    -> WorldData
```

Starting positions are planned as Stage 3.6 but are not currently part of
`generate_world`.

## Documents

| Document | Purpose |
|----------|---------|
| [api.md](api.md) | Public API reference for callers. |
| [pipeline.md](pipeline.md) | Overall pipeline and per-stage data flow. |
| [parameters/index.md](parameters/index.md) | Tuning-parameter index. |
| [parameters/tectonics.md](parameters/tectonics.md) | Tectonics tuning reference. |
| [parameters/climate.md](parameters/climate.md) | Climate tuning reference. |
| [parameters/hydrology.md](parameters/hydrology.md) | Hydrology tuning reference. |
| [parameters/biomes.md](parameters/biomes.md) | Biome tuning reference. |
| [parameters/resources.md](parameters/resources.md) | Resource tuning reference. |
| [data-assets.md](data-assets.md) | RON asset schemas for biome and resource config. |

## Source Of Truth

The code remains authoritative for behavior:

| Topic | Authoritative source |
|-------|----------------------|
| Public exports | `crates/worldgen/src/lib.rs` |
| Input parameters | `crates/worldgen/src/params.rs` and `crates/worldgen/src/params/` |
| Pipeline order | `crates/worldgen/src/lib.rs::generate_world` |
| Stage implementation | `crates/worldgen/src/stages/` |
| Output data layout | `crates/worldgen/src/world_data.rs` |
| Biome table | `assets/worldgen/biome_table.ron` |
| Resource config | `assets/worldgen/resources.ron` |

When a `pub` item is added to the `pub use` block in `lib.rs`, update
[api.md](api.md) in the same change. When a tuning field or asset schema changes,
update the corresponding parameter or data-asset document.

