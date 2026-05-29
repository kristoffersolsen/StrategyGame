# `tools/mapbench` — Implementation Plan

## Overview

`mapbench` is a CLI binary that automatically runs a configurable matrix of
(seed, map size, renderer) cases, scripting camera movement through each scene,
and collecting rendering performance statistics. Output is a human-readable
terminal table and a machine-readable JSON file so that results are archivable
and comparable across time as the game grows in complexity.

Scope: dev tool only. No interaction required after launch. Must produce
reproducible, comparable numbers that survive being run months apart.

---

## Why a separate process per case

Each benchmark case requires a full fresh Bevy context: asset loading,
GPU resource allocation, and chunk spawning all happen at startup. Attempting
to re-initialize within a single Bevy run requires carefully despawning all
entities and resources and re-triggering one-shot systems — fragile, and it
would pollute the startup measurements with leftover GPU state.

Instead, `mapbench` in orchestrator mode spawns itself as a subprocess once per
case with `--single-case`. Each subprocess:
1. Times world generation (before Bevy starts)
2. Times chunk spawning (state transition duration)
3. Runs the scripted camera path for 30 s, collecting frame-time samples
4. Prints a JSON `CaseResult` to stdout, then exits

The orchestrator collects stdout from each subprocess, aggregates results,
prints a summary table to stderr, and writes the full JSON report to disk.

Startup overhead per case (~5–10 s on current hardware) is acceptable because
the tool is run occasionally, not in CI on every commit.

---

## Architecture

```
tools/mapbench/
├── Cargo.toml
└── src/
    ├── main.rs            ← CLI dispatch: orchestrator or single-case
    ├── config.rs          ← BenchConfig, BenchCase RON types
    ├── orchestrator.rs    ← spawns subprocesses, aggregates, reports
    ├── single_case.rs     ← Bevy app setup for one case
    ├── camera_path.rs     ← scripted orbit camera system
    ├── collector.rs       ← frame-time & RSS sampling, timing markers
    └── reporter.rs        ← terminal table + JSON output formatting
```

---

## Dependency additions (`tools/mapbench/Cargo.toml`)

```toml
[dependencies]
game      = { path = "../../crates/game" }
worldgen  = { path = "../../crates/worldgen" }
bevy      = { workspace = true }
clap      = { workspace = true }
ron       = { workspace = true }
serde     = { workspace = true, features = ["derive"] }
anyhow    = { workspace = true }
tracing   = { workspace = true }
tracing-subscriber = { workspace = true }
```

`game` is the only new dependency not already used by `maprender`. No new
external crates are needed; RSS is read from `/proc/self/status` via
`std::fs::read_to_string`.

---

## CLI interface

```
mapbench [OPTIONS] [CONFIG]

Arguments:
  [CONFIG]               RON bench config file [default: tools/mapbench/bench.ron]

Options:
  --output <DIR>         Directory for JSON result files [default: bench-results]
  --seed <SEED>          Override all seeds with a single value
  --size <SIZE>          Override all map sizes (Small/Standard/Medium/Large/Huge)
  --duration <SECS>      Override measurement duration per case [default: 30]
  --single-case <JSON>   Internal: run one case and print CaseResult JSON to stdout
```

`--single-case` is an internal flag used by the orchestrator; users never
call it directly.

---

## Configuration (`bench.ron`)

```rust
// tools/mapbench/bench.ron  (default config)
BenchConfig(
    seeds: [1, 2, 3, 4],
    sizes: [Small, Standard, Medium, Large, Huge],
    renderer: Terrain3d,          // P3 only for now
    measurement_secs: 30,
    output_dir: "bench-results",
)
```

`BenchConfig` is defined in `config.rs` and derives `serde::Deserialize`.
The orchestrator cross-products `seeds × sizes` to build the case list.
When `--seed` or `--size` overrides are provided via CLI, they replace
the config list.

---

## Metrics collected per case

| Metric | Source | Captured where |
|--------|--------|----------------|
| `world_gen_ms` | `Instant::elapsed()` around `generate_world()` | Before Bevy starts, in `single_case.rs` |
| `chunk_spawn_ms` | `Instant` at `OnEnter(SpawningChunks)` → `OnEnter(Running)` | `collector.rs` timing markers |
| `rss_mb` | `/proc/self/status` → `VmRSS` line | Sampled once at 5 s into measurement phase |
| `frame_time_ms.{mean, min, max, variance, p95, p99}` | `FrameTimeDiagnosticsPlugin::FRAME_TIME` via `DiagnosticsStore` | Collected each frame in `collector.rs` |
| `fps.{mean, min, max}` | Derived from frame time samples | Computed in `reporter.rs` at end of case |

`FrameTimeDiagnosticsPlugin` is already registered by `RenderingPlugin` in the
`game` crate. The collector reads raw `Measurement` values (not the smoothed
display value) to get per-frame samples.

Variance is the unbiased sample variance; p95/p99 are computed by sorting the
sample vector. Total sample count is also recorded.

---

## Camera scripted path

The benchmark uses the 3D orbit camera (`CameraOrbit` resource from
`game::rendering::camera`). The scripted path is implemented as a Bevy system
in `camera_path.rs` that runs in `Update` during the measurement phase. It
directly mutates `CameraOrbit` rather than synthesizing input events, so it is
not affected by key-repeat delays or input focus.

```
t = 0 – 10 s  : Pan east — pivot.x increases at (map_world_width / 10) units/s
t = 10 – 20 s : Pan west — pivot.x decreases at same rate (return traversal)
t = 20 – 25 s : Zoom in  — distance decreases from 900 to 300 linearly
t = 25 – 30 s : Zoom out — distance increases from 300 to 900 linearly
```

`yaw` and `pitch` are held at their initial values (`-π/2`, `π/3.5`) throughout,
giving a consistent viewpoint and avoiding measuring rotation cost.

Pivot clamping (east-west wrap) follows the same logic as `pan_camera()` in
`camera.rs`, ensuring the camera never leaves the valid world-space bounds.

---

## Output formats

### Terminal (stderr, printed by orchestrator after all cases)

```
mapbench results — 2026-05-28 14:33:02 — git da32e7e

Size       Seed  GenMs  SpawnMs  RSS_MB  FPS_mean  FPS_min  FT_p99ms
─────────────────────────────────────────────────────────────────────
Small         1   45.2   823.1    312.4      61.2     44.1      24.8
Small         2   44.9   819.7    311.8      61.5     43.2      25.1
Medium        1  234.5  1820.3    512.4      59.8     31.2      32.1
...
Huge          4  891.2  6234.0   1821.7      28.3      9.4     111.4
```

### JSON (written to `bench-results/YYYY-MM-DDTHH-MM-SS.json`)

```json
{
  "timestamp": "2026-05-28T14:33:02Z",
  "git_hash": "da32e7e",
  "renderer": "Terrain3d",
  "measurement_secs": 30,
  "results": [
    {
      "seed": 1,
      "map_size": "Medium",
      "world_gen_ms": 234.5,
      "chunk_spawn_ms": 1820.3,
      "rss_mb": 512.4,
      "frame_time_ms": {
        "mean": 16.7,
        "min": 14.2,
        "max": 32.1,
        "variance": 4.3,
        "p95": 28.5,
        "p99": 32.1,
        "samples": 1789
      },
      "fps": { "mean": 59.8, "min": 31.2, "max": 70.4 }
    }
  ]
}
```

The `git_hash` is obtained by running `git rev-parse --short HEAD` at startup
via `std::process::Command`. If git is unavailable (clean release build, CI
artifact), the field is `"unknown"`.

---

## Module responsibilities

### `config.rs`

`BenchConfig` (RON-deserialized) and `BenchCase` (derived by cross-product).
`RendererArg` mirrors the one in `maprender` — share the type if feasible
without adding a new crate dependency.

### `orchestrator.rs`

Iterates cases, spawns `std::process::Command` of the current executable with
`--single-case <json>`, captures stdout, deserializes `CaseResult`, and hands
the vec to `reporter.rs`. Cases run sequentially (one GPU at a time). Prints
progress to stderr as each case completes.

### `single_case.rs`

Times `generate_world()`, constructs a `RenderStartupConfig` matching
`maprender`'s pattern, builds a Bevy `App` with:
- `RenderingPlugin` (from `game` crate, already contains `FrameTimeDiagnosticsPlugin`)
- `BenchPlugin` (local plugin registering collector + camera path systems)

On startup, the app enters the existing `LoadingAssets → SpawningChunks →
Running` state machine. `BenchPlugin` adds systems to:
- Record `chunk_spawn_start` on `OnEnter(SpawningChunks)` (via `Local<Option<Instant>>`)
- Record `chunk_spawn_end` on `OnEnter(Running)` and start the measurement timer
- Drive the camera path and collect samples in `Update(Running)`
- Exit the app after `measurement_secs` via `AppExit` event

Serializes the final `CaseResult` to stdout as JSON before exiting.

### `camera_path.rs`

Single Bevy system `drive_camera_path(time: Res<Time>, orbit: ResMut<CameraOrbit>, bounds: Res<MapRenderBounds>, phase: Res<BenchMeasurePhase>)`.

Reads elapsed measurement time, applies the four-phase motion described above.
`MapRenderBounds` (already computed by `chunk_spawner.rs`) provides the
world-space map width for scaling the pan speed.

### `collector.rs`

System `collect_frame_metrics(diagnostics: Res<DiagnosticsStore>, mut state: ResMut<CollectorState>)` runs each frame in `Update(Running)`. Reads
`FrameTimeDiagnosticsPlugin::FRAME_TIME` measurements from `DiagnosticsStore`
and appends to a `Vec<f64>`. Also records the RSS sample at the 5 s mark.

`CollectorState` holds:
- `samples: Vec<f64>` (frame time in ms)
- `rss_mb: Option<f64>`
- `chunk_spawn_ms: f64` (written by `single_case.rs` on `OnEnter(Running)`)

### `reporter.rs`

Pure functions: `compute_stats(samples: &[f64]) -> FrameStats` (mean, min, max,
variance, p95, p99) and formatting helpers for the terminal table and JSON.

---

## Implementation sequence

### Step 1 — Scaffold (2–3 hours)

- Create `tools/mapbench/` with `Cargo.toml` (game + worldgen + clap + ron + serde + anyhow)
- Add `mapbench` to workspace `Cargo.toml` members
- Stub `main.rs` with CLI arg parsing; `--single-case` prints `"not implemented"` and exits 0
- Confirm `cargo build -p mapbench` and `cargo clippy -p mapbench` are clean

**Gate:** binary compiles, `--help` shows correct usage.

### Step 2 — Single-case runner (half day)

- Implement `single_case.rs`: world gen timing + Bevy app setup mirroring `maprender`
- Implement stub `BenchPlugin` that exits the app after 5 s (no collection yet)
- Implement `camera_path.rs` system (mutates `CameraOrbit` per the four phases)
- Confirm the window opens, camera moves across the map, and the app exits cleanly

**Gate:** `mapbench --single-case <json>` opens a window, moves camera, exits.

### Step 3 — Metric collection (half day)

- Implement `collector.rs` with frame-time sampling and RSS reading
- Wire `chunk_spawn_ms` timing markers in `single_case.rs`
- Implement `reporter.rs` `compute_stats()` and JSON serialization
- `--single-case` now prints a valid `CaseResult` JSON to stdout

**Gate:** `mapbench --single-case <json> | jq .` shows well-formed metrics.

### Step 4 — Orchestrator (half day)

- Implement `orchestrator.rs`: config loading, case iteration, subprocess spawning,
  stdout aggregation
- Implement terminal table formatting in `reporter.rs`
- Implement JSON file writing (with git hash, timestamp, full result list)
- Test with a 2-seed × 2-size config

**Gate:** `cargo run --release -p mapbench` completes all cases and writes
`bench-results/*.json` with correct data.

### Step 5 — Defaults and docs (2 hours)

- Write `tools/mapbench/bench.ron` default config (4 seeds × 5 sizes × Terrain3d)
- Write `tools/mapbench/README.md` covering invocation, output interpretation,
  and how to add a new renderer to the matrix when P1/P2 coverage is desired
- Add `mapbench` entry to `documents/planning/implementation-plan.md`

**Gate:** `cargo run --release -p mapbench -- --duration 5` completes in under
2 minutes with plausible numbers.

---

## Verification

```bash
# Quick smoke test (5 s per case, 2 seeds × 2 sizes)
cargo run --release -p mapbench -- \
    --size small --size medium --seed 1 --seed 2 --duration 5

# Check JSON output is well-formed
cat bench-results/$(ls -t bench-results | head -1) | jq .

# Check terminal table is printed to stderr
cargo run --release -p mapbench -- --duration 5 2>&1 | grep -E "FPS|mean"

# Full default run
cargo run --release -p mapbench
```

Expected output: terminal table with plausible FPS values (>30 on Medium,
dropping on Huge), JSON file with all fields populated, no panics.

---

## What this is not

- Not a CI gate — run times are too long and require a GPU. Intended for
  manual regression checks before and after a significant feature addition.
- Not a replacement for Bevy's built-in `FrameTimeDiagnosticsPlugin` overlay —
  that remains the real-time tool; this is the archivable batch tool.
- Not a load generator for AI or pathfinding — those are pure-CPU concerns that
  belong in `cargo bench` microbenchmarks once those systems exist.
- Not shipped or compiled for release builds. Stays in `tools/` as a workspace
  member that the game binary never depends on.
