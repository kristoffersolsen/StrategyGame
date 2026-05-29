# `tools/maptune` — Implementation Plan

## Overview

`maptune` is a native egui/eframe GUI binary in the workspace for interactive
world-generation parameter tuning. It calls into `worldgen` directly (no
subprocess), renders output images inline, and replaces the edit-RON → run-CLI
→ open-file-explorer loop with a single window.

Scope: dev tool only. No prettiness requirement. Must be correct, fast, and
easy to extend as new parameters and figure types are added.

---

## Architecture

```
tools/maptune/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs            ← eframe entry point, app bootstrap
    ├── app.rs             ← AppState, top-level update() / draw()
    ├── params_panel.rs    ← scrollable parameter form (tectonics + climate)
    ├── sweep_panel.rs     ← sweep config form (param, min, max, steps, seeds)
    ├── figure_panel.rs    ← figure selector + inline PNG display
    ├── runner.rs          ← background generation thread, channel-based
    └── figures.rs         ← SINGLE REGISTRY: all known figure names + render fns
```

The key insight for maintainability: **`figures.rs` is the only file that
changes when a new output type is added.** Everything else reads from it.

---

## Dependency additions (`Cargo.toml`)

```toml
[dependencies]
eframe      = "0.29"      # includes egui
egui_extras = "0.29"      # RetainedImage / image loading
image       = "0.25"      # already in worldgen; load PNG bytes → egui texture
worldgen    = { path = "../../crates/worldgen" }
anyhow      = { workspace = true }
crossbeam-channel = "0.5" # background thread result passing
```

`eframe` pulls in `egui`. `egui_extras` provides `RetainedImage` for
displaying in-memory PNG buffers without writing to disk. `crossbeam-channel`
(or `std::sync::mpsc`) passes generation results from the worker thread back
to the UI thread without blocking the frame loop.

The `image` crate is already a transitive dependency of `worldgen`; add it
explicitly to `maptune`'s `Cargo.toml` to pin it.

Note: `eframe` uses `winit` under the hood. On Linux you need either
`x11-dev` / `libxkbcommon-dev` or Wayland equivalents. Add a note to
`CONTRIBUTING.md`.

---

## Module responsibilities

### `figures.rs` — the registry (the pipeline-maintenance anchor)

```rust
//! Registry of all available debug figure types.
//!
//! To add a new figure: add one entry to `ALL_FIGURES` and implement
//! the corresponding `render_*` function. Nothing else needs changing.

pub struct FigureDescriptor {
    /// Stable snake_case identifier used as the dropdown label.
    pub id: &'static str,
    /// Human-readable label shown in the selector.
    pub label: &'static str,
    /// Render the figure to a raw RGBA byte buffer given generated world data.
    pub render: fn(world: &WorldOutput) -> Vec<u8>,
}

pub const ALL_FIGURES: &[FigureDescriptor] = &[
    FigureDescriptor { id: "height_levels_hex",  label: "Height levels (hex)",    render: render_height_levels_hex  },
    FigureDescriptor { id: "plate_map_hex",       label: "Plate map (hex)",        render: render_plate_map_hex      },
    FigureDescriptor { id: "boundary_types",      label: "Boundary types",         render: render_boundary_types     },
    FigureDescriptor { id: "elevation",           label: "Elevation",              render: render_elevation          },
    FigureDescriptor { id: "hills",               label: "Hills",                  render: render_hills              },
    FigureDescriptor { id: "temperature_hex",     label: "Temperature (hex)",      render: render_temperature_hex    },
    FigureDescriptor { id: "wind",                label: "Wind magnitude",         render: render_wind               },
    FigureDescriptor { id: "wind_ns",             label: "Wind N-S component",     render: render_wind_ns            },
    FigureDescriptor { id: "moisture",            label: "Moisture",               render: render_moisture           },
    FigureDescriptor { id: "precipitation_hex",   label: "Precipitation (hex)",    render: render_precipitation_hex  },
    FigureDescriptor { id: "precipitation_log",   label: "Precipitation (log)",    render: render_precipitation_log  },
    // Add new entries here. That's the entire maintenance cost.
];
```

The `render` functions call the existing `debug_viz` helpers from `worldgen`
and return raw RGBA bytes. `figure_panel.rs` converts those bytes to an
`egui` texture and displays them. No figure-specific logic lives outside
this file.

`WorldOutput` is a thin struct holding `&TectonicData` and `&ClimateData`
(and later `&HydrologyData` etc.). It grows with the pipeline.

### `runner.rs` — background generation

Generation is not instant on large maps. Running it on the UI thread freezes
the frame loop. The runner owns a worker thread and two channels:

```
UI thread                        Worker thread
─────────────────────────────    ──────────────────────────────
RunRequest → tx_request ──────►  receive WorldParams, run generate_world()
             rx_result  ◄──────  send Result<WorldOutput, GenerationError>
```

The UI sends a `WorldParams` snapshot on "Run" or "Sweep" button press.
The worker sends back either a completed result or an error string. The UI
checks `rx_result.try_recv()` each frame — no blocking.

For sweep mode the worker sends back a `Vec<WorldOutput>` (one per step),
or streams them one by one if that feels faster in practice.

### `params_panel.rs` — the parameter form

Renders two collapsible sections: **Tectonics** and **Climate**. Each field
is a row:

```
[label]    [drag-slider or int spinner]    [reset-to-default button]
```

The form binds directly to a `TuningParams` struct held in `AppState`.
It does not parse RON; it mutates the struct in memory. A "Export as RON"
button serializes it to a file chooser for saving named presets (reuses
`ron::to_string`). A "Load RON" button deserializes from a file.

`level_proportions` and `mineral_weights` (the `Vec<f32>` fields) get a
fixed-count row of drag-sliders — not a scalar knob.

Fields that have validated hard bounds (`wind_amplitude ≤ 0.25`, etc.) show
the slider clamped to those bounds, with the bound value shown as a tooltip.

### `sweep_panel.rs` — sweep config

Shown/hidden by a toggle. When shown:

```
Sweep parameter: [dropdown of all scalar TuningParams field names]
Min: [f64 input]    Max: [f64 input]    Steps: [int input, min 2]
Seeds: [multi-value int list]
```

On "Run Sweep", sends a `SweepRequest` to the runner, which reuses the same
`set_param` logic from `tools/maptest/src/sweep.rs`. The sweep is run on
the worker thread via rayon (same as `maptest sweep`).

### `figure_panel.rs` — image display

Left sidebar: a list of figure names from `ALL_FIGURES`. The selected figure
is highlighted.

Right panel: displays the rendered texture. If a sweep is active, shows a
grid (one image per step value, label = the parameter value).

Zoom: scroll wheel zooms the image; click-and-drag pans. egui provides
`Image::fit_to_available_width` as the default.

### `app.rs` — state and layout

```rust
struct AppState {
    params: TuningParams,
    seed: u64,
    map_size: MapSize,
    hex_size: u32,
    sweep_config: Option<SweepConfig>,
    selected_figure: usize,           // index into ALL_FIGURES
    last_result: Option<WorldOutput>, // last successful single run
    sweep_results: Vec<(f32, WorldOutput)>, // (param_value, output)
    status: StatusMessage,            // "Running…" / "Done in 1.2s" / error
    runner: Runner,
}
```

Layout: three-panel horizontal split.
```
┌─────────────────┬────────────────────────────────┬──────────┐
│  Params panel   │       Figure display           │ [future] │
│  (scrollable)   │                                │          │
│                 │                                │          │
│─────────────────│                                │          │
│  Sweep panel    │                                │          │
│  (collapsible)  │                                │          │
│─────────────────│                                │          │
│  [Run] [Sweep]  │                                │          │
│  [Export RON]   │                                │          │
│  [Load RON]     │                                │          │
│  Status bar     │                                │          │
└─────────────────┴────────────────────────────────┴──────────┘
                   Figure selector tabs (top of right panel)
```

---

## Implementation sequence

### Step 1 — Scaffold (half day)

- Add `tools/maptune/` with `Cargo.toml` and stub `main.rs`
- Add `maptune` to workspace `Cargo.toml` members
- Confirm `cargo build -p maptune` compiles with an empty eframe window
- Write `tools/maptune/README.md`

**Gate:** empty window opens, `cargo clippy -p maptune` clean.

### Step 2 — Parameter form (half day)

- Implement `params_panel.rs` with all tectonics + climate scalar fields
- Implement `figures.rs` with `ALL_FIGURES` registry and RGBA render stubs
  (solid-color placeholders — not yet calling worldgen)
- Implement `app.rs` `AppState` struct and the three-panel layout skeleton
- Export-to-RON and load-from-RON buttons functional

**Gate:** form renders, all sliders update `AppState`, RON round-trip works.

### Step 3 — Runner + single-run display (half day)

- Implement `runner.rs` with worker thread and channels
- Wire "Run" button → `WorldParams` snapshot → worker → result
- Implement real `render_*` functions in `figures.rs` (calls `debug_viz`
  helpers, returns RGBA bytes)
- `figure_panel.rs` loads RGBA bytes as an egui texture and displays it
- Status bar shows "Running…" / "Done in Xs" / error message

**Gate:** press Run, see a real map image in the window within a few seconds.

### Step 4 — Sweep mode (half day)

- Implement `sweep_panel.rs`: parameter dropdown, min/max/steps inputs
- Wire "Run Sweep" → `SweepRequest` → worker (uses rayon, reuses `set_param`)
- `figure_panel.rs` sweep view: grid of images, labeled with parameter value
- Sweep results stored in `AppState`; figure selector applies to all images

**Gate:** sweep run produces a grid of images in the window.

### Step 5 — Polish and docs (quarter day)

- Reset-to-default buttons on each parameter row
- Tooltip on each parameter label (copy from `tectonics-parameters.md`)
- ADR-0010: rationale for egui over alternatives, tool scope decision
- Update `documents/architecture/architecture.md` with `maptune` entry
- Update `CONTRIBUTING.md` with platform dependencies (X11/Wayland)

---

## The maintenance pipeline

When a new parameter is added to `TuningParams`:

1. Add the field to the relevant `params/*.rs` struct (already required for
   the feature to work).
2. Add a row to `params_panel.rs` in the correct section. One line for
   scalar fields:
   ```rust
   scalar_row(ui, "new_field_name", &mut params.climate.new_field, 0.0..=1.0);
   ```
3. Add the field name to `sweep.rs` `set_param` (already required for
   `maptest sweep` to work).
4. Add a preset entry to `tools/maptest/presets/default.ron`.

When a new figure type is added:

1. Add the render function to `output.rs` in `maptest` (already required
   for `maptest run` to produce the image).
2. Add one entry to `ALL_FIGURES` in `figures.rs`. That's it.
   `figure_panel.rs` reads `ALL_FIGURES` at runtime — no other changes.

The rule: **no figure logic outside `figures.rs`, no parameter logic outside
`params_panel.rs` rows**. If a change requires touching `app.rs` or
`figure_panel.rs` for a routine parameter/figure addition, something has
drifted from this structure.

---

## What this is not

- Not a replacement for `maptest run` or `maptest sweep` — those remain the
  canonical batch/CI tools. `maptune` is for interactive iteration only.
- Not a preset manager — save/load RON covers that need adequately.
- Not shipped or compiled for release builds of the game. Kept in `tools/`
  behind a workspace member that the game binary never depends on.
