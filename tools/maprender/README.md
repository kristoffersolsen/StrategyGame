# maprender

Interactive map-rendering harness for the Bevy terrain renderer.

`maprender` launches the reusable `game::rendering` plugin against a generated
world, or launches isolated rendering prototypes. The generated-map path shows
the chunked continuous terrain surface with camera, lighting, and diagnostics.

Use `mapdump` for static PNGs, `maptest` for multi-seed worldgen comparisons,
and `maprender` when you need the Bevy renderer or a rendering prototype.

## Quick Start

```bash
# Run the generated-map renderer:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/default.ron

# Run the R1 three-tier corner prototype suite:
cargo run --release -p maprender -- --prototype corners

# Same preset, different seed:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/default.ron --seed 42

# Same preset, larger map:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/default.ron --size large
```

Run performance measurements with `--release`; debug builds are useful for
development but are not valid render-gate measurements.

## Command

```text
cargo run --release -p maprender -- [OPTIONS]

Options:
  --preset <FILE>        RON file with RenderPreset
                         [default: tools/maprender/presets/default.ron]
  --seed <N>             Override the preset seed
  --size <SIZE>          Override the preset map size:
                         small | standard | medium | large | huge
  --prototype <MODE>     Launch an isolated prototype:
                         corners
```

## Preset Files

A preset file is a RON-serialized `RenderPreset`:

```ron
(
    seed: 1,
    map_size: Medium,
    tuning: (
        tectonics: (
            target_plate_count: 17,
        ),
    ),
)
```

Fields:

| Field | Meaning |
|---|---|
| `seed` | World-generation seed |
| `map_size` | `Small`, `Standard`, `Medium`, `Large`, or `Huge` |
| `tuning` | `worldgen::TuningParams` overrides |

## R1 Prototype

`--prototype corners` renders the isolated three-tier corner suite. It covers
equal tiers, one-step slopes, multi-step cliffs, and forced-cliff arrangements
around each edge. Use this harness for the visual sign-off required before the
full chunked terrain lattice is rebuilt.
