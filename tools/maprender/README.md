# maprender

Interactive map-rendering performance gate for the Bevy terrain renderer.

`maprender` launches the reusable `game::rendering` plugin against a
generated world. It is a tool, not the final game. Use it to validate
rendering performance, compare rendering backends, and tune worldgen
parameters while keeping production game startup free of hardcoded test
maps.

Use `mapdump` when you want static PNGs for one seed. Use `maptest` when
you want multi-seed worldgen comparisons or parameter sweeps. Use
`maprender` when you want to run the actual Bevy renderer interactively.

---

## Quick Start

```bash
# Run the default P1 vertex-colour renderer preset:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/default.ron

# Run the P2 texture-array renderer preset:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/p2_texture_array.ron

# Same preset, different seed:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/default.ron --seed 42

# Same preset, larger map:
cargo run --release -p maprender -- \
    --preset tools/maprender/presets/default.ron --size large

# Show available CLI options:
cargo run -p maprender -- --help
```

Run performance measurements with `--release`; debug builds are useful
for development but are not valid render-gate measurements.

---

## Command

```
cargo run --release -p maprender -- [OPTIONS]

Options:
  --preset <FILE>        RON file with RenderPreset
                         [default: tools/maprender/presets/default.ron]
  --seed <N>             Override the preset seed
  --size <SIZE>          Override the preset map size:
                         small | standard | medium | large | huge
  --renderer <RENDERER>  Override the preset renderer:
                         vertex-color | texture-array
```

The CLI overrides only the named field. All other values still come from
the preset file.

---

## Controls

| Input | Action |
|---|---|
| `W` / `ArrowUp` | Pan north |
| `A` / `ArrowLeft` | Pan west |
| `S` / `ArrowDown` | Pan south |
| `D` / `ArrowRight` | Pan east |
| Mouse wheel | Zoom in/out |

The camera is clamped to the rectangular rendered map footprint. Seamless
east-west cylindrical panning is future gameplay-camera work.

The top-left overlay shows smoothed FPS and frame time.

---

## Preset Files

A preset file is a RON-serialized `RenderPreset`:

```ron
(
    seed: 1,
    map_size: Medium,
    renderer: VertexColor,
    tuning: (
        tectonics: (
            target_plate_count: 17,
        ),
        // omitted tuning sections and fields use worldgen defaults
    ),
)
```

Fields:

| Field | Meaning |
|---|---|
| `seed` | World-generation seed |
| `map_size` | `Small`, `Standard`, `Medium`, `Large`, or `Huge` |
| `renderer` | Terrain rendering backend |
| `tuning` | `worldgen::TuningParams` overrides |

`tools/maprender/presets/default.ron` documents the P1 vertex-colour
baseline. `tools/maprender/presets/p2_texture_array.ron` is the P2
texture-array gate preset. Copy either file to create named render-test
presets.

```bash
cp tools/maprender/presets/default.ron tools/maprender/presets/large_vertex_color.ron
```

---

## Renderer Modes

| Preset value | CLI value | Status |
|---|---|---|
| `VertexColor` | `vertex-color` | Implemented. P1 baseline: flat terrain colours encoded as mesh vertex colours. |
| `TextureArray` | `texture-array` | Implemented. P2 baseline: terrain layer indices encoded as mesh attributes and sampled from a 64x64 placeholder texture array. |

Renderer selection lives in `game::rendering::RenderStartupConfig`.
This keeps render tests and the final game compatible: the final game can
choose a renderer from game state, while `maprender` chooses from a RON
preset.

---

## Recording A Gate Run

For P1, record results in `documents/adr/0018-map-rendering-chunk-rendering-approach.md`.
For P2, record results in `documents/adr/0019-map-rendering-texture-pipeline.md`.

Minimum useful run:

1. Run `cargo run --release -p maprender -- --preset tools/maprender/presets/default.ron`.
2. For P2, run `cargo run --release -p maprender -- --preset tools/maprender/presets/p2_texture_array.ron`.
3. Pan continuously across the full map width at least twice.
4. Record minimum FPS, average FPS, maximum frame-time spike, and peak memory.
5. Note any Bevy validation warnings or startup errors.

The pass criteria are documented in `documents/planning/map-rendering-plan.md`.

---

## Adding New Render Tests

When a new rendering stage lands:

1. Add the renderer implementation in `crates/game/src/rendering/`.
2. Add or activate the corresponding `TerrainRenderer` variant in
   `game::rendering`.
3. Add a preset under `tools/maprender/presets/`.
4. Keep old presets working unless the underlying renderer was
   intentionally removed.
5. Add a parsing test if the preset introduces new required fields.

This keeps P1/P2/P3 gate runs comparable over time and avoids coupling
the performance harness to final game startup.
