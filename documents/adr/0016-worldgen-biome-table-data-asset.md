# ADR-0016: Biome Table as a Data Asset

## Status

Accepted

## Context

Stage 3.4 assigns land biomes from the climate stage's continuous
temperature and moisture fields. The lookup table is designer-editable,
has dozens of cells, and is coupled to hydrology override mappings.

Embedding that table in Rust code would require recompilation for
tuning. Embedding it in `BiomeParams` would make maptest scalar sweeps
and preset files noisy and hard to review.

## Decision

Store the biome lookup table as committed RON data at
`assets/worldgen/biome_table.ron`.

`WorldParams` owns the table path, defaulting to that asset. At pipeline
startup, `generate_world` loads and validates the table before running
expensive generation stages. Validation rejects:

- table bin counts that do not match `BiomeParams`
- out-of-range `(temperature_bin, moisture_bin)` entries
- duplicate entries
- missing entries in the configured grid

Hydrology overrides are stored in the same table as a
`BTreeMap<Biome, Biome>` so iteration order remains deterministic.

## Consequences

Biome tuning can happen by editing a data file rather than recompiling
Rust logic. The bin-count fields in `BiomeParams` are intentionally not
freely sweepable; changing either one requires regenerating the table to
match.

The table loader is now a load-time invariant for world generation:
broken biome data fails fast with `GenerationError::InvalidBiomeTable`.
