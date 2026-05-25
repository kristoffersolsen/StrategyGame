# ADR-0003: Parameter Tuning Infrastructure — `maptest` Crate and RON Preset Files

**Status:** Accepted  
**Date:** 2026-05-25  
**Deciders:** Kristoffer

## Context

Stage 3.1 (tectonics) introduced roughly two dozen tunable parameters
(`TectonicsParams`). Tuning a procedural generation pipeline requires
running many (parameter, seed) combinations and inspecting the visual
output. We needed tooling to support this workflow.

The initial approach was to expose every parameter as a CLI flag on
`mapdump`. By the end of Stage 3.1 this had grown to 27 flags. The
problems with that approach:

1. **Shell scripts become load-bearing.** Named configurations
   ("high mountain density", "archipelago") have to live in shell
   scripts that pass dozens of `--flag value` pairs. These are fragile,
   hard to diff, and not composable.

2. **Multi-seed and sweep runs are not first-class.** Running seeds 1–8
   with the same parameters, or sweeping `mountain_elevation_boost` from
   0.4 to 2.0 in 9 steps across 4 seeds, requires a wrapper script with
   an outer loop — boilerplate that every developer has to re-implement.

3. **No parallelism.** Shell loops are sequential; a 9×4 sweep (36 jobs)
   takes 36× the time of a single run.

4. **Does not scale.** Stages 3.2–3.6 will add more parameter structs.
   Each new parameter would require a new CLI flag, a new shell-script
   variable, and a new case in any sweep helper.

5. **Colormap code was duplicated or inline.** Debug visualization
   helpers (`BoundaryCmap`, `HeightLevelCmap`) had no natural home and
   were defined locally in `mapdump`, unavailable to any second tool.

## Decision

### 1. `TectonicsParams` derives `serde::Serialize` / `serde::Deserialize`

This is the enabling change. It costs two derive attributes and makes
the entire params struct round-trippable through any serde-compatible
format. We use **RON** (Rusty Object Notation) because:

- Struct fields are named, not positional (unlike TOML arrays or JSON
  without schema), so preset files are self-documenting.
- RON supports inline comments (`//`), which are used to annotate each
  pipeline step in the preset.
- RON is already a workspace dependency (`serde` derives are also
  already present in the workspace).

Named presets live in `tools/maptest/presets/`. The default preset,
`presets/default.ron`, documents every field and its default value and
serves as the authoritative reference for new presets.

### 2. A dedicated `tools/maptest` crate replaces shell scripts

`maptest` is a Rust binary crate with two subcommands:

- **`run`** — generate debug images for a fixed parameter set across
  multiple seeds. Equivalent to the old `tune_mapgen.sh` but type-safe,
  composable with `--params`, and without shell-script boilerplate.

- **`sweep`** — vary one `TectonicsParams` scalar field across an
  evenly-spaced range of values and generate all (value, seed) pairs.
  Field names are matched by string at runtime (`set_param` function);
  unknown names produce a descriptive error listing all valid names.

The crate is not a library; it has no public API (`#![allow(missing_docs)]`).

### 3. `rayon` parallelism in `sweep`

Sweep jobs are embarrassingly parallel. `tasks.par_iter().try_for_each`
runs all (value, seed) pairs across the rayon thread pool. On a
6-core machine a 9×4 sweep runs in roughly `9×4 / 6 ≈ 6×` wall time
rather than `36×`. `--jobs` allows capping parallelism explicitly.

Output subdirectories are created once on the main thread before the
parallel loop (`ensure_output_dirs`), so no filesystem races occur.

### 4. Shared colormap types move to `worldgen::debug_viz`

`BoundaryCmap`, `HeightLevelCmap`, and the existing `Viridis`,
`Categorical`, `Grayscale` colormaps all live in `worldgen::debug_viz`.
Both `mapdump` and `maptest` import from there. Adding a new colormap
in one place makes it available to both tools automatically.

### 5. `mapdump` is reduced to five flags

After the above changes, `mapdump` needs only: `--seed`, `--output`,
`--size`, `--hex-size`, `--params`. It remains the right tool for
inspecting a single (seed, params) pair; `maptest` handles everything
that needs a loop or parallelism.

The 27 parameter flags and the `apply_tectonics_overrides` function
are deleted entirely.

## Consequences

**Positive:**

- Named presets are first-class: stored as RON files, diffable, shareable,
  composable with both tools via `--params <file.ron>`.
- Sweep runs are fast: `rayon` parallelism scales to available cores
  without any manual threading in userland code.
- Adding a new generation stage's parameters to the tuning workflow
  requires only: serde derives on the new params struct, new match arms
  in `set_param`, and new image-writing calls in `output.rs`. No
  shell-script changes, no CLI flag explosion.
- `mapdump` is clean and readable again.
- Colormap code has one canonical location.

**Negative:**

- `set_param` maps field names as strings at runtime. If a field is
  renamed in `TectonicsParams`, the match arm must be updated manually;
  the compiler will not catch the mismatch. (The probe validation at
  sweep startup catches unknown names before any work is done.)
- RON is a less common format than TOML or JSON; contributors unfamiliar
  with it must read `presets/default.ron` to understand the syntax.
  In practice the syntax is close enough to Rust struct literals that
  the learning curve is minimal.

## Alternatives considered

**Shell scripts wrapping `mapdump --flag value ...`** — the status quo.
Rejected because it does not solve the parallelism problem, does not
provide first-class named configurations, and scales poorly as parameter
count grows.

**TOML or JSON for preset files** — both are valid serde targets.
Rejected in favour of RON because RON supports comments (essential for
documenting what each parameter does inline in the preset file) and
because the struct-literal syntax mirrors Rust more directly.

**Proc-macro reflection for `set_param`** — deriving a `set_by_name`
method automatically from the struct fields, eliminating the match arm
maintenance burden. Rejected as over-engineering for the current scale;
the match arm list is ~25 lines and is straightforward to maintain.

**`mapdump` as the single tool with a `--sweep` mode** — adding sweep
directly to `mapdump` rather than a separate crate. Rejected because
sweep requires `rayon` as a dependency (heavy), a second subcommand
structure, and output organization that differs from single-seed dumps.
Keeping the tools separate keeps each one focused.

## References

- `tools/maptest/README.md` — usage guide
- `tools/maptest/presets/default.ron` — reference preset with all defaults
- `tools/maptest/src/sweep.rs` — `set_param` implementation and `linspace`
- `tools/maptest/src/output.rs` — output directory structure
- `crates/worldgen/src/debug_viz.rs` — shared colormap types
- ADR-0001 — RNG threading (context for why tuning one stage must not
  shift others, motivating the need for a reliable tuning workflow)
