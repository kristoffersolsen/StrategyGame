# worldgen public API

Reference for calling code (tools, game crate, tests). Read this instead of the source
when you need to know what exists — then open the source only for stage internals.

Staleness note: the authoritative source is `crates/worldgen/src/lib.rs`. When a `pub`
item is added to the `pub use` block there, add a line here in the same PR.

---

## Entry point

```rust
pub fn generate_world(params: &WorldParams) -> Result<WorldData, GenerationError>
```

Single entry point used by `game` (at startup) and all tools (headlessly). Returns a
complete `WorldData` or a `GenerationError`. Same `params` always produces byte-identical
output.

Asset paths in `WorldParams` default to relative paths (`assets/worldgen/biome_table.ron`,
`assets/worldgen/resources.ron`). The working directory must be the workspace root, or the
paths must be overridden.

---

## WorldParams

```rust
pub struct WorldParams {
    pub seed: u64,
    pub basic: BasicParams,
    pub advanced: AdvancedParams,
    pub tuning: TuningParams,
    pub biome_table_path: PathBuf,       // default: "assets/worldgen/biome_table.ron"
    pub resource_config_path: PathBuf,   // default: "assets/worldgen/resources.ron"
}
```

`WorldParams::default()` / `WorldParams::defaults()` → medium continents map, seed 0,
4 players, standard climate and resources.

### BasicParams (flavor layer)

```rust
pub struct BasicParams {
    pub map_size: MapSize,       // Small/Standard/Medium/Large/Huge/Custom{width,height}
    pub archetype: MapArchetype, // Pangaea/Continents/Archipelago/Islands
}
```

`MapSize` concrete dimensions: Small 120×80, Standard 180×120, **Medium 240×150** (default),
Large 320×200, Huge 400×250.

### AdvancedParams (strategic layer)

```rust
pub struct AdvancedParams {
    pub land_fraction: f32,              // [0.0, 1.0], recommended 0.3–0.6, default 0.4
    pub climate_band: ClimateBand,       // Cold/Temperate(default)/Hot
    pub world_age: WorldAge,             // Young/Standard(default)/Old
    pub resource_abundance: ResourceAbundance, // Scarce/Standard(default)/Abundant
    pub player_count: u32,               // ≥ 1, default 4
}
```

### TuningParams (internal knobs)

```rust
pub struct TuningParams {
    pub tectonics: TectonicsParams,
    pub climate: ClimateParams,
    pub hydrology: HydrologyParams,
    pub biomes: BiomeParams,
    pub resources: ResourceParams,
}
```

Setting any field to non-default disables achievements. Per-stage parameter details:

| Stage | Struct | Reference doc |
|-------|--------|---------------|
| 3.1 Tectonics | `TectonicsParams` | `documents/crates/worldgen/parameters/tectonics.md` |
| 3.2 Climate | `ClimateParams` | `documents/crates/worldgen/parameters/climate.md` |
| 3.3 Hydrology | `HydrologyParams` | `documents/crates/worldgen/parameters/hydrology.md` |
| 3.4 Biomes | `BiomeParams` | `documents/crates/worldgen/parameters/biomes.md` |
| 3.5 Resources | `ResourceParams` | `documents/crates/worldgen/parameters/resources.md` |

Key constants exposed from `TectonicsParams`:
`MIN_TARGET_PLATE_COUNT`, `MAX_TARGET_PLATE_COUNT`, `MIN_LAND_LEVEL_COUNT`,
`MAX_LAND_LEVEL_COUNT`, `MIN_DETAIL_NOISE_OCTAVES`, `MAX_DETAIL_NOISE_OCTAVES`.  
From climate: `CFL_MAX_WIND_AMPLITUDE`.

---

## WorldData

```rust
pub struct WorldData {   // #[non_exhaustive]
    pub bounds: MapBounds,       // required to interpret any Field
    pub tectonics: TectonicData,
    pub climate: ClimateData,
    pub hydrology: HydrologyData,
    pub biomes: BiomeData,
    pub resources: ResourceData,
}
```

### TectonicData (stage 3.1)

| Field | Type | Contents |
|-------|------|----------|
| `seeds` | `Vec<Coord>` | Plate seed coordinates; index = `PlateId` |
| `plate_id` | `Field<PlateId>` | Per-tile plate assignment |
| `plates` | `Vec<Plate>` | Per-plate `{plate_type, motion, mineral_character}` |
| `boundary_type` | `Field<Option<BoundaryType>>` | None = interior; Some = boundary tile type |
| `elevation` | `Field<f32>` | Continuous elevation, sea level = 0, range ≈ ±1.5 |
| `height_level` | `Field<u8>` | Discrete levels: 0=ocean, 1=coast, 2..N=land, N+1=mountain |
| `is_hilly` | `Field<bool>` | Combat/movement modifier |

`Plate` fields: `plate_type: PlateType` (Continental/Oceanic), `motion: MotionVector {dx, dy}`,
`mineral_character: MineralCharacter` (Ferrous/Precious/Gems/Hydrocarbons/Industrial).

`BoundaryType`: Convergent / Divergent / Transform.

### ClimateData (stage 3.2)

| Field | Type | Contents |
|-------|------|----------|
| `temperature` | `Field<f32>` | Normalised; range ≈ [−0.5, 1.0] at defaults |
| `moisture` | `Field<f32>` | Accumulated evaporation minus precipitation |
| `wind` | `Field<[f32; 2]>` | `[east, south]` velocity in hex Cartesian frame |
| `precipitation` | `Field<f32>` | Summed across all advection iterations |

### HydrologyData (stage 3.3)

| Field | Type | Contents |
|-------|------|----------|
| `flow_direction` | `Field<Option<Direction>>` | None = terminal (ocean/coast/lake) |
| `flow_accumulation` | `Field<f32>` | Normalised to [0.0, 1.0] by map-wide max |
| `is_lake` | `Field<bool>` | Inland basin filled by sink resolution |
| `lake_outlet` | `Field<Option<Coord>>` | Spill point for each lake tile |
| `river_size` | `Field<Option<RiverSize>>` | Creek / River / MajorRiver |
| `is_floodplain` | `Field<bool>` | Adjacent to river, low elevation |
| `is_canyon` | `Field<bool>` | Canyon-carved river tile |
| `is_canyon_wall` | `Field<bool>` | Flanking a carved canyon |
| `height_level` | `Field<u8>` | Post-hydrology copy for debug comparison (ADR-0012) |

### BiomeData (stage 3.4)

| Field | Type | Contents |
|-------|------|----------|
| `biome` | `Field<Option<Biome>>` | None for ocean/coast/lake/mountain |
| `forest_density` | `Field<f32>` | [0.0, 1.0] rendering hint; 0 for non-forest tiles |

`Biome`: TropicalRainforest / TemperateForest / BorealForest / Grassland / Desert / Tundra.

### ResourceData (stage 3.5)

| Field | Type | Contents |
|-------|------|----------|
| `resource` | `Field<Option<ResourceId>>` | One resource per tile max; None = empty |

`ResourceId`: Iron / Horses / Gold / Gems / Spices / Wine / Wheat / Fish / Stone.
Full list: `ResourceId::ALL`.

---

## Field\<T\>

Dense per-tile storage, row-major `(r * width + q)`. Indexed by `Coord` + implicit `MapBounds`.

```rust
Field::filled(bounds, value: T) -> Field<T>  // fill all tiles with value; T: Clone
Field::from_fn(bounds, f: FnMut(Coord) -> T) -> Field<T>  // row-major init

field.get(coord: Coord) -> Option<&T>    // None if r out of bounds; q wraps
field.set(coord: Coord, value: T)        // panics if r out of bounds
field[coord]                             // Index/IndexMut: panics on out-of-bounds

field.iter_coords() -> impl Iterator<Item=Coord>  // row-major, deterministic order
field.bounds() -> MapBounds
```

`q` wraps silently in all accessors; `r` is bounded and returns `None` / panics.

---

## RNG model

```rust
// In worldgen::rng (pub mod)
pub struct MasterRng { ... }
pub enum StageId { Tectonics, Climate, Hydrology, Biomes, Resources }

MasterRng::new(seed: u64) -> MasterRng
master.fork(stage: StageId) -> StageRng   // idempotent; same stage always gives same sub-RNG
```

Each stage calls `master.fork(StageId::X)` to get its own sub-RNG. Tuning stage N's
parameters never shifts stage M's output. Full strategy in ADR-0001.

Workspace rule: `rand::thread_rng` / `rand::rng` are forbidden everywhere.

---

## Error types

```rust
pub enum GenerationError {  // #[non_exhaustive]
    InvalidParams { reason: String },             // bad WorldParams (validated before pipeline)
    InvalidBiomeTable { path: PathBuf, reason },  // biome_table.ron missing or malformed
    InvalidResourceConfig { path: PathBuf, ... }, // resources.ron missing or malformed
    AdequacyFailure { players: u32 },             // stage 3.6: can't seat N players (not yet reachable)
}
```

---

## Other exports

```rust
// Re-exported from coords
pub use coords::Direction;  // East/West/NorthEast/NorthWest/SouthEast/SouthWest

// Debug visualization (pub mod debug_viz)
// Colormap helpers and Field→PNG rasterization — used by mapdump/maptest
```
