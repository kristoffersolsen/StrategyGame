# mapbench

Automated rendering benchmark for the strategy game.

`mapbench` runs a configurable matrix of (seed × map-size) cases through the
Bevy renderer, scripts the camera through a deterministic path, and records
frame-time statistics. Results go to a terminal table and a timestamped JSON
file so runs are archivable and comparable over time.

Use `maprender` when you want to interact with a specific map. Use `mapbench`
when you want to track rendering performance trends — before and after a
significant feature addition, or when something feels slower than it used to.

---

## Quick start

```bash
# Quick smoke run: 2 sizes, 2 seeds, 5-second window each:
cargo run --release -p mapbench -- --size small --size medium --seed 1 --seed 2 --duration 5

# Full default run (5 sizes × 4 seeds × 30 s each, ~30 min):
cargo run --release -p mapbench

# Full run with a custom config file:
cargo run --release -p mapbench -- tools/mapbench/bench.ron

# Override just the seeds:
cargo run --release -p mapbench -- --seed 1 --seed 2 --seed 3
```

Always use `--release`. Debug builds are too slow to produce meaningful numbers.

---

## Command

```
cargo run --release -p mapbench -- [OPTIONS] [CONFIG]

Arguments:
  [CONFIG]          RON bench configuration file
                    [default: tools/mapbench/bench.ron]

Options:
  --output <DIR>    Directory for JSON result files [default: bench-results]
  --seed <N>        Override all seeds (repeatable)
  --size <SIZE>     Override all map sizes (repeatable):
                    small | standard | medium | large | huge
  --duration <N>    Override measurement duration in seconds
```

CLI overrides replace the entire seed or size list from the config file. To
test one specific combination, pass both `--seed` and `--size` once each.

---

## Config file

`tools/mapbench/bench.ron` controls the default test matrix:

```ron
BenchConfig(
    seeds: [1, 2, 3, 4],
    sizes: [Small, Standard, Medium, Large, Huge],
    renderer: Terrain3d,
    measurement_secs: 30,
    output_dir: "bench-results",
)
```

| Field | Meaning |
|---|---|
| `seeds` | World-generation seeds to benchmark |
| `sizes` | Map sizes to benchmark; each crossed with every seed |
| `renderer` | Terrain renderer: `Terrain3d`, `VertexColor`, or `TextureArray` |
| `measurement_secs` | Steady-state camera-movement window per case |
| `output_dir` | Directory for JSON result files |

Any field can be omitted; missing fields use the defaults above.

---

## What it measures

For each (seed × size) case, `mapbench` records:

| Metric | Description |
|---|---|
| `world_gen_ms` | CPU-only world generation time, measured in isolation before the Bevy app starts |
| `startup_ms` | Time from `App::run()` to the first camera frame — includes asset loading, mesh build, and GPU upload |
| `rss_mb` | Resident set size at 5 s into the measurement window |
| `frame_time_ms.mean` | Average frame duration over the measurement window |
| `frame_time_ms.min` | Fastest observed frame |
| `frame_time_ms.max` | Slowest observed frame |
| `frame_time_ms.variance` | Unbiased sample variance — measures frame-time consistency |
| `frame_time_ms.p95` | 95th-percentile frame time — typical worst-case |
| `frame_time_ms.p99` | 99th-percentile frame time — detects hitching even when mean FPS looks fine |
| `fps.mean` | Mean FPS |
| `fps.min` | Minimum FPS (corresponds to the slowest frame) |

---

## Camera path

During each measurement window the camera follows a scripted four-phase orbit
path, repeatable across runs on the same hardware:

| Phase | Duration | Movement |
|---|---|---|
| 1 | 0–10 s | Pan east: map centre → right edge |
| 2 | 10–20 s | Pan west: right edge → left edge |
| 3 | 20–25 s | Zoom in: distance 900 → 300 |
| 4 | 25–30 s | Zoom out: distance 300 → 900 |

This covers the full map width at two zoom levels. Yaw and pitch are held
constant so the only variable is the pivot position and distance.

---

## Output

### Terminal table (stderr)

After all cases finish, a summary table is printed:

```
mapbench results — renderer: Terrain3d — 5s per case

Size            Seed     GenMs  StartupMs    RSS_MB  FPS_mean   FPS_min  FT_p99ms
────────────────────────────────────────────────────────────────────────────────
Small              1      44.9      823.1     312.4      61.5      43.2      25.1
Medium             1     234.5     1820.3     512.4      59.8      31.2      32.1
```

### JSON file (bench-results/`<timestamp>`.json)

```json
{
  "timestamp": "2026-05-28T14:33:02Z",
  "git_hash": "da32e7e",
  "renderer": "Terrain3d",
  "measurement_secs": 5,
  "results": [
    {
      "seed": 1,
      "map_size": "Medium",
      "world_gen_ms": 234.5,
      "startup_ms": 1820.3,
      "rss_mb": 512.4,
      "frame_time_ms": {
        "mean": 16.7, "min": 14.2, "max": 32.1,
        "variance": 4.3, "p95": 28.5, "p99": 32.1,
        "samples": 299
      },
      "fps": { "mean": 59.8, "min": 31.2, "max": 70.4 }
    }
  ]
}
```

JSON files are written to `bench-results/` (or the configured `output_dir`)
with the ISO 8601 timestamp as the filename. Keep a few historical runs to
identify regressions.

---

## How to track regressions

Run `mapbench` before and after a significant change (new feature, renderer
update, large refactor) and compare the JSON files:

```bash
# Before the change:
cargo run --release -p mapbench -- --duration 10

# Make your changes, then:
cargo run --release -p mapbench -- --duration 10

# Compare the two most recent results:
diff <(jq '.results[] | {map_size, fps_mean: .fps.mean, p99: .frame_time_ms.p99}' \
        bench-results/$(ls -t bench-results | sed -n '2p')) \
     <(jq '.results[] | {map_size, fps_mean: .fps.mean, p99: .frame_time_ms.p99}' \
        bench-results/$(ls -t bench-results | head -1))
```

The key signals to watch:

- **`startup_ms` on Huge** — most sensitive to world-gen cost; rises with new
  per-tile data.
- **`frame_time_ms.p99`** — detects hitching that the mean hides; relevant once
  entities move or AI ticks.
- **`rss_mb` on Huge** — tracks memory growth as entity count grows.

---

## Architecture note

Each (seed × size) case runs as a separate subprocess. The orchestrator spawns
the same binary with a hidden `--single-case` flag and reads the JSON result
from stdout. This gives each case a clean GPU context and a fresh Bevy
startup — important for accurate `startup_ms` measurements.

`world_gen_ms` is measured in the orchestrator process before launching the
subprocess, so it is a pure CPU timing with no GPU noise.
