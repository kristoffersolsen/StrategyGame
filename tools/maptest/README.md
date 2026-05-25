# maptest

Parameter tuning and visual regression tool for world generation.

Two subcommands: **`run`** generates debug images for a fixed parameter
set across multiple seeds; **`sweep`** varies a single scalar parameter
across a range of values and generates all (value, seed) pairs in
parallel.

Use `mapdump` when you want to inspect a single seed. Use `maptest`
when you want to compare seeds or tune a parameter.

---

## Quick start

```bash
# Multi-seed run with defaults (seeds 1–8):
cargo run --release --bin maptest -- run

# Same with a named preset:
cargo run --release --bin maptest -- run \
    --params tools/maptest/presets/default.ron

# Sweep mountain_elevation_boost from 0.4 to 2.0 in 9 steps, seeds 1–4:
cargo run --release --bin maptest -- sweep \
    --param mountain_elevation_boost --min 0.4 --max 2.0 --steps 9

# Sweep with a custom base config, 8 parallel jobs:
cargo run --release --bin maptest -- sweep \
    --param boundary_perturbation_amplitude --min 0 --max 16 --steps 9 \
    --params tools/maptest/presets/default.ron --jobs 8
```

---

## Subcommands

### `run`

Generate the full set of debug images for a fixed parameter set across
multiple seeds.

```
cargo run --release --bin maptest -- run [OPTIONS]

Options:
  --seeds <N>...          Seeds to generate [default: 1 2 3 4 5 6 7 8]
  --params <FILE>         RON file with TectonicsParams [default: built-in defaults]
  --output <DIR>          Output directory [default: tmp/tests/mapgen]
  --size <SIZE>           Map size: small | standard | medium | large | huge [default: medium]
  --hex-size <PX>         Hex tile circumradius in pixels [default: 16]
```

Output files are written to `<output>/` with filenames like
`31_08_height_levels_hex_seed_3.png`. All seeds share the same output
directory; filename prefixes distinguish the pipeline stage.

### `sweep`

Vary a single `TectonicsParams` scalar field across an evenly-spaced
range and generate all (value, seed) pairs in parallel.

```
cargo run --release --bin maptest -- sweep [OPTIONS]

Required:
  --param <NAME>          TectonicsParams field name in snake_case
  --min <F>               Range minimum (inclusive)
  --max <F>               Range maximum (inclusive)
  --steps <N>             Number of values to generate (minimum 2)

Options:
  --seeds <N>...          Seeds per value [default: 1 2 3 4]
  --params <FILE>         RON file with base TectonicsParams
  --output <DIR>          Output directory [default: tmp/tests/tuning/<param>]
  --size <SIZE>           Map size [default: medium]
  --hex-size <PX>         Hex tile circumradius in pixels [default: 16]
  --jobs <N>              Max parallel jobs [default: rayon thread pool size]
```

Output files are written to `<output>/` with filenames like
`31_08_height_levels_hex_v0.4000_3.png`, where `v0.4000` is the
parameter value and `3` is the seed.

Vec fields (`level_proportions`, `mineral_weights`) cannot be swept as
scalars; set them via `--params` instead.

---

## Output files

Each (value, seed) pair produces this set of images:

| File prefix | Content |
|---|---|
| `31_01_plate_seeds_px` | Plate seed positions (white dots on black) |
| `31_03_plate_map_px` | Plate map, one pixel per tile, categorical colors |
| `31_03_plate_map_hex` | Plate map, hex-rasterized |
| `31_05_boundary_types_px` | Boundary classification (red=convergent, blue=divergent, grey=transform) |
| `31_07_elevation_px` | Raw elevation field, normalized to [0,1], Viridis colormap |
| `31_08_height_levels_px` | Discrete height levels, one pixel per tile |
| `31_08_height_levels_hex` | Discrete height levels, hex-rasterized |
| `31_09_hills_px` | Hilliness field (white=hilly, black=flat) |

File prefixes are the pipeline step number so files sort in generation
order in any file browser.

---

## Preset files

A preset file is a RON-serialized `TectonicsParams` struct. Any field
omitted from the file takes the built-in default value.

```ron
// tools/maptest/presets/my-preset.ron
(
    target_plate_count: 14,
    boundary_perturbation_amplitude: 10.0,
    mountain_elevation_boost: 1.6,
    // all other fields use defaults
)
```

Pass it to either subcommand with `--params`:

```bash
cargo run --release --bin maptest -- run \
    --params tools/maptest/presets/my-preset.ron
```

`tools/maptest/presets/default.ron` documents every field with its
default value and the pipeline step it belongs to. Copy it as a
starting point for new presets.

---

## Sweepable fields

All scalar fields of `TectonicsParams` can be passed to `--param`:

```
target_plate_count            boundary_perturbation_amplitude
motion_magnitude_min          motion_magnitude_max
convergence_threshold         continental_base_elevation
oceanic_base_elevation        intra_plate_noise_amplitude
intra_plate_noise_frequency   boundary_falloff_sigma
mountain_elevation_boost      coastal_mountain_boost
subduction_trench_penalty     island_arc_boost
rift_valley_penalty           mid_ocean_ridge_boost
detail_noise_amplitude        detail_noise_frequency
detail_noise_octaves          land_level_count
hill_influence_radius         hill_near_boundary_threshold
hill_interior_threshold       hill_noise_frequency
```

Integer fields (`target_plate_count`, `land_level_count`,
`detail_noise_octaves`, `hill_influence_radius`) are rounded before
casting, so `--min 2 --max 5 --steps 4` gives values 2, 3, 4, 5.

---

## Adding support for new generation stages

When a new stage adds a new params struct:

1. Add `#[derive(serde::Serialize, serde::Deserialize)]` to the struct.
2. Add match arms for the new fields in `tools/maptest/src/sweep.rs` →
   `set_param`.
3. Add image-writing calls in `tools/maptest/src/output.rs` →
   `write_world_outputs`.
4. Add a new preset file documenting the new fields, or update
   `presets/default.ron` to include them.

No changes to the subcommand CLI are needed.
