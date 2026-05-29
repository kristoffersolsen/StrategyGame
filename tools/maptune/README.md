# maptune

Interactive GUI tool for world-generation parameter tuning.

Calls into `worldgen` directly, renders debug figures inline, and
replaces the edit-RON → run-CLI → open-file-explorer loop with a
single window. Use `maptest` for batch runs and sweep-to-disk; use
`maptune` for interactive iteration.

---

## Quick start

```bash
cargo run --release --bin maptune
```

Requires X11 or Wayland on Linux. See **Platform dependencies** below.

---

## Layout

```
┌─────────────────┬──────────────────────────────────────────────┐
│  Params panel   │  [Height levels (hex)] [Plate map] [Elev] …  │
│  (scrollable)   │                                               │
│                 │                                               │
│  Sweep panel    │            figure display                     │
│  (collapsible)  │                                               │
│─────────────────│                                               │
│  [Run] [Sweep]  │                                               │
│  [Export RON]   │                                               │
│  [Load RON]     │                                               │
│  Status bar     │                                               │
└─────────────────┴───────────────────────────────────────────────┘
```

**Left panel** — scrollable form with collapsible sections for Global,
Tectonics, Climate, Hydrology, Biomes, and Resources parameters. Each
field has a drag-slider and a `↺` reset-to-default button.

**Figure tabs** — select which debug figure to display. The selected
figure is rendered lazily and cached until parameters change or a new
run completes.

**Sweep grid** — when a sweep run completes, the figure display shows
one image per sweep step, each labeled with its parameter value.

---

## Single run

1. Adjust seed, map size, hex size, and any tuning parameters.
2. Press **Run**.
3. The status bar shows "Running…" while generation is in progress on
   a background thread. On completion it shows "Done in Xs".
4. Click any figure tab to see that figure for the generated world.

---

## Sweep mode

1. Check **Sweep mode** to reveal the sweep panel.
2. Choose the parameter to vary from the dropdown.
3. Set **Min**, **Max**, and **Steps** (minimum 2).
4. Optionally add extra seeds (comma-separated); each step uses the
   base seed from the Global section, plus any extra seeds listed.
5. Press **Run Sweep**.
6. The figure display shows a grid of images, one per step, labeled
   with the parameter value.

The sweep runs in parallel via rayon. The base seed provides one image
per step for the grid. Vec and bool fields are not sweepable; set them
via Load RON instead.

---

## RON presets

**Export RON** — saves the current `TuningParams` to a `.ron` file via
a file chooser. Use this to snapshot a configuration worth keeping.

**Load RON** — loads a `TuningParams` from a `.ron` file. Fields
absent from the file take their built-in defaults. Compatible with
preset files from `maptest`.

```ron
// Example preset
(
    tectonics: (
        target_plate_count: 14,
        mountain_elevation_boost: 1.6,
    ),
    // all other fields use defaults
)
```

---

## Available figures

| Figure | Stage | Renderer |
|---|---|---|
| Height levels (hex) | Tectonics | hex |
| Plate map (hex) | Tectonics | hex |
| Boundary types | Tectonics | pixel |
| Elevation | Tectonics | pixel |
| Hills | Tectonics | pixel |
| Temperature (hex) | Climate | hex |
| Wind magnitude | Climate | pixel |
| Wind N-S component | Climate | pixel |
| Moisture | Climate | pixel |
| Precipitation (hex) | Climate | hex |
| Precipitation (log) | Climate | hex |
| Flow direction | Hydrology | hex |
| Flow accumulation | Hydrology | hex |
| Lakes | Hydrology | hex |
| Rivers | Hydrology | hex |
| Floodplains | Hydrology | pixel |
| Canyons | Hydrology | pixel |
| Hydrology overlay | Hydrology | hex |
| Biomes | Biomes | hex |
| Forest density | Biomes | pixel |
| Resources | Resources | hex |

---

## Platform dependencies

On Linux, eframe requires one of:

- **X11**: `libx11-dev`, `libxkbcommon-dev`
- **Wayland**: `libwayland-dev`, `libxkbcommon-dev`

Install via your package manager, e.g.:

```bash
# Ubuntu / Debian
sudo apt install libx11-dev libxkbcommon-dev

# Arch
sudo pacman -S libx11 libxkbcommon
```

---

## Adding a new figure

1. Add the render function to `worldgen/src/debug_viz.rs` (or
   implement it inline in the render fn below).
2. Add one entry to `ALL_FIGURES` in `src/figures.rs` and implement
   the `render_*` function. That is the entire maintenance cost —
   `figure_panel.rs` reads `ALL_FIGURES` at runtime.

No changes to `app.rs`, `figure_panel.rs`, or any other file are
needed for a routine figure addition.

## Adding a new sweepable parameter

1. Add a match arm to `src/sweep.rs` → `set_param`.
2. Add the field name to `SWEEPABLE_PARAMS` in `src/sweep.rs`.
3. Add a row to the appropriate section in `src/params_panel.rs`.

Also update `tools/maptest/src/sweep.rs` → `set_param` to keep the
two tools in sync.
