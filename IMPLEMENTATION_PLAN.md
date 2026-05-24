# Implementation Plan

This document defines the staged execution plan for the Strategy Game
project. It complements the design doc (which describes *what* and *why*)
by specifying *in what order* and *with what gates*.

The plan is a **living document** through Stage 3. Stages 4 and beyond
are placeholders that will be planned in detail when the prerequisite
work and design decisions are settled.

## Principles

A few principles that the staging reflects.

**Stages have explicit completion gates.** Each stage defines what must
be true before the next can begin. Gates are not "I think it's done";
they are concrete, verifiable conditions. If a gate isn't met,
subsequent stages don't start. This prevents the "everything is 80%
done and nothing actually works" failure mode.

**Infrastructure before features.** Generation tooling before
generation stages. Determinism testing before generation logic. Debug
visualization before parameter tuning. The temptation to skip
infrastructure for "real work" is the project's biggest sequencing
risk — see the Stage 2 notes for why.

**Visual validation drives generation work.** Every generation stage
ships with a PNG dump that lets you eyeball the output. Tuning
parameters by reading numbers is unworkable; tuning by looking at
images is fast. Stage 2 builds this tooling first because every
subsequent stage depends on it.

**One thing at a time.** A PR implements one stage's slice. The
PR_CHECKLIST is the enforcement point. Splitting stages into multiple
PRs is fine; combining them into one is not — large PRs accumulate
unverified assumptions that bite later.

**Defer the undecided.** Stages 4+ involve design topics that are still
open (design doc §4 roadmap). Planning them in detail before the
design questions are resolved is premature. They're sketched here
as placeholders, not committed plans.

---

## Stage 0: Foundations

**Goal:** Establish the development environment, workspace structure,
documentation scaffolding, and git workflow before any code is written.

This stage exists because retrofitting discipline after the fact is
expensive. The CI checks, commit conventions, and PR workflow defined
here apply to every subsequent commit, including the first.

### 0.1 Workspace skeleton

- Create three empty crates: `coords`, `worldgen`, `game`.
- Create one binary tool crate: `mapdump`.
- Configure `Cargo.toml` workspace manifest with shared
  `[workspace.dependencies]` and `[workspace.lints]` (clippy pedantic
  + the additional rustc lints from style guide §1.3).
- Pin `rust-toolchain.toml` to Rust 1.95.0, edition 2024.
- Add `rustfmt.toml` (defaults) and `.editorconfig`.
- Commit `Cargo.lock`.

Each empty crate has at minimum:
- `Cargo.toml` referencing workspace dependencies
- `src/lib.rs` (or `src/main.rs` for `mapdump`) with the module-level
  `//!` doc explaining the crate's purpose

### 0.2 Foundational ADRs

Write the ADRs that record decisions already made in planning. Do
these as one batched PR — they're records, not active decisions.

- ADR-0001: Workspace structure (three library crates + CLI tool)
- ADR-0002: Forbid `unsafe_code` at workspace level
- ADR-0003: Hybrid error handling (`thiserror` in library crates,
  `anyhow` at application boundaries)
- ADR-0004: Pin Rust 1.95.0 + Bevy 0.18, update deliberately
- ADR-0005: RON for config files (per design doc references)
- ADR-0006: Conventional Commits adopted for git history
- ADR-0007: Trunk-based development; all changes to main via PR

Each ADR follows the template in documentation guide §4.4. The ADR
index (`docs/adr/README.md`) is created and populated.

### 0.3 Operational documentation

- `README.md` at repo root: one-paragraph description, status, quick
  start, doc index, license placeholder.
- `CONTRIBUTING.md` at repo root: toolchain setup, build/test commands,
  style and lint enforcement, commit message conventions, branch
  strategy, ADR/design-doc conventions. References the existing style
  and documentation guides.
- Move existing documents into place:
  - `STYLE_GUIDE.md` at repo root
  - `DOCUMENTATION_GUIDE.md` at repo root
  - `PR_CHECKLIST.md` at repo root
  - `docs/design/overview.md` (currently `design-decisions.md`,
    renamed and moved)
  - `.github/pull_request_template.md` (the PR template)

Note: `ARCHITECTURE.md` is deferred to the end of Stage 1, when there
is actual architecture to describe.

### 0.4 CI configuration

A GitHub Actions workflow that runs on every PR:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps` (catches broken rustdoc links)

Failure on any of these blocks the PR.

### 0.5 Git hooks and commit linting

Set up local pre-commit hooks for:
- `cargo fmt --check` (so format errors are caught before push)
- Commit message linting against the Conventional Commits format
- TODO comment hygiene check (every `TODO` has a `(name): description`
  pattern)

**Open sub-decision:** hook framework choice (`pre-commit`,
`cargo-husky`, hand-rolled). To be resolved as part of the development
environment setup; will become ADR-0008.

### 0.6 Initial repo state validation

Before declaring Stage 0 complete: make a trivial PR (e.g., adding a
TODO to a stub file) and verify the full workflow:
- Pre-commit hooks run locally
- CI runs on the PR
- PR template auto-populates
- All checks pass
- Merge to main works cleanly

This is the dress rehearsal for every subsequent PR.

### Gate: Stage 0 complete

All of the following must be true:
- Workspace builds clean with `cargo build --workspace`.
- `cargo clippy --workspace -- -D warnings` passes.
- All ADRs 0001–0007 (and 0008 for the hook framework) are committed
  and in the index.
- All operational docs exist and link to each other correctly.
- A trial PR has been opened and merged successfully exercising the
  full workflow.
- The PR_CHECKLIST.md template is in `.github/` and auto-populates.

---

## Stage 1: The `coords` crate

**Goal:** Wrap-aware hexagonal coordinate math, usable by both
generation and (eventually) runtime.

This is the first crate because everything spatial depends on it, and
because it's pure Rust with no external dependencies — a clean way to
validate the workflow established in Stage 0 against real code.

### 1.1 Core type and constructors

- `Coord` (axial representation: `q`, `r`).
- Constructors with bounds awareness.
- `Display` and `Debug` impls.
- Module-level `//!` doc establishing the wrap-and-bounds semantics
  (style guide §9).

### 1.2 Map context

A `MapBounds` (or similar) type holding width and height. The wrap
behavior is contextual — a `Coord` doesn't wrap in isolation, it
wraps relative to a map. Decide concretely whether wrap operations
take `MapBounds` as a parameter or whether `Coord` carries a reference
to it.

Recommendation: pass `MapBounds` explicitly. Avoids lifetimes on
`Coord` and is the simpler ownership story. Style guide hot-loop
exception (`_unchecked` variants) applies if profiling later shows it
matters.

### 1.3 Arithmetic and queries

- Neighbor enumeration (6 directions for hex, wrap-aware in E/W,
  bounded in N/S — returns `Option<Coord>` per direction).
- Distance between two coords (wrap-aware: shortest path may go
  through the wrap seam).
- Range queries: tiles within N steps of a center.
- Wrap normalization: given any coord, return its canonical
  in-bounds form.

Note: cube coordinate conversion and line-of-sight queries are
**deferred to Stage 1+** — added when the first consumer needs them.
Avoid designing in a vacuum.

### 1.4 Testing

- Unit tests in `#[cfg(test)] mod tests` blocks for individual
  operations.
- Integration tests in `crates/coords/tests/`:
  - Wrap idempotency: wrapping twice equals wrapping once
  - Distance symmetry: `distance(a, b) == distance(b, a)` for all pairs
  - Range query consistency: distance to every coord in `range(c, n)`
    is `<= n`
- Property tests with `proptest` for the wrap/distance invariants.

Test coverage is not the metric — these are the *interesting*
properties that catch bugs. Adding cosmetic tests for full coverage is
busywork.

### 1.5 Documentation

- Rustdoc on every `pub` item (enforced by lint).
- `# Examples` on the primary entry points (`Coord::new`,
  `Coord::neighbors`, `Coord::distance`, `Coord::range`).
- Module-level `//!` doc covers the wrap semantics in detail.
- Crate-level `README.md` in `crates/coords/`.

### Gate: Stage 1 complete

- All operations from §1.1–1.3 implemented.
- All tests from §1.4 passing on a clean checkout.
- Property tests verifying invariants over generated inputs.
- Full rustdoc on public API.
- `ARCHITECTURE.md` written now that there's structure to describe.

---

## Stage 2: Worldgen infrastructure

**Goal:** Build the supporting infrastructure that every generation
stage will depend on, *before* writing any generation logic.

This stage is where most projects fail by skipping ahead. The design
doc is explicit (§1.16): debug visualization, determinism testing, and
the standalone CLI runner must exist before stage 1 of generation is
implemented. Tuning generation parameters by running the full Bevy
game and squinting at on-screen tiles is unworkable; tuning by
generating a PNG in 5 seconds and looking at it is fast. This
infrastructure pays for itself within the first parameter sweep.

### 2.1 Shared types

In the `worldgen` crate root:
- `WorldData` — the finalized generation output (per design doc §1.8).
  Holds heightmap, climate fields, biome map, resources, starting
  positions. Read by the `game` crate to spawn entities.
- `WorldParams` — generation input parameters. Maps to the basic +
  advanced + full-tunability layers from design doc §1.15.
- `GenerationError` — typed error enum (`thiserror`), with variants
  for the failure modes we know about: invalid params, adequacy
  failure at starting position selection, etc.
- A `Field<T>` or similar container for the per-tile fields used
  during generation (elevation, temperature, moisture). Indexed by
  `Coord` (Stage 1 dependency).

These are the API contract for every generation stage. Get them
roughly right now; iterate as stages reveal requirements.

### 2.2 The `mapdump` CLI tool

`crates/.../mapdump` binary:
- Takes a seed and parameter overrides on the command line.
- Calls into the `worldgen` crate's `generate_world` function (which
  will be a stub initially that returns an empty `WorldData`).
- Dumps intermediate and final fields to PNG files in a specified
  output directory.

Why this matters: every subsequent generation stage is tuned by
running `mapdump --seed X --param Y` and looking at the resulting PNGs.
This must work end-to-end before the first generation stage is
implemented.

Initially, `mapdump` produces empty/dummy PNGs (the generation stub
returns nothing). The point is the *plumbing* — argument parsing,
output directory handling, PNG writing, file naming.

### 2.3 Debug visualization (PNG dumps)

A small module in `worldgen` (or in a sibling crate if it grows) that:
- Renders any `Field<T>` to a PNG, given a colormap.
- Supports common colormaps: grayscale (for heightmap), viridis (for
  scalar fields like moisture), categorical (for biomes, terrain).
- Handles hex tiling correctly — output PNGs should look like a hex
  grid, not a square grid pretending.

This is the load-bearing tool. Every parameter tuning decision comes
through these PNGs. Make it good.

### 2.4 Determinism test scaffold

Integration tests in `crates/worldgen/tests/`:
- `same_seed_produces_identical_world_data` — generate twice with the
  same seed, byte-compare output.
- `different_seeds_produce_different_world_data` — sanity check.
- A reference seed that produces a known-hash output, committed to
  the repo. Any change to generation that affects this hash is
  flagged for review (the change might be intentional, but it must
  be acknowledged).

The scaffold is built now so that every generation stage adds to it,
catching determinism regressions at the stage that introduces them.

### 2.5 Single seeded RNG strategy

Establish the pattern used by every generation stage:
- One master `rand_chacha::ChaCha8Rng` seeded from the input seed.
- Each stage derives a sub-RNG from the master with a fixed
  stage-identifier salt. Tweaking stage N's parameters doesn't shift
  stage M's output for M > N.
- No `thread_rng` anywhere in the workspace. Add a clippy lint
  override if needed to enforce this.

Document this pattern in the `worldgen` module-level `//!` doc, since
every stage must follow it.

### Gate: Stage 2 complete

- `WorldData`, `WorldParams`, `GenerationError` types defined and
  documented.
- `mapdump` CLI builds, accepts a seed argument, writes PNG files
  to a configurable output directory.
- PNG dump module renders a sample `Field<f32>` to a viridis-colored
  hex grid (using a hand-constructed test field — no generation needed).
- Determinism test scaffold runs (passing trivially against the stub
  generation).
- Master/sub-RNG pattern is documented and used by the stub.
- ADR-0009 written: determinism strategy and RNG threading.

---

## Stage 3: Generation pipeline

**Goal:** Implement the six generation stages from the design doc,
in pipeline order, with visual validation between each.

This is the substantial body of generation work. Each sub-stage is
its own milestone — multiple PRs each, several days to weeks of
tuning per stage. The design is settled (design doc §1.9–1.14); the
work is implementation.

The pattern for each sub-stage:
1. Implement the algorithm per the design doc.
2. Run `mapdump` over a range of seeds. Look at the PNGs.
3. Tune parameters until output looks geographically plausible.
4. Add stage-specific determinism tests.
5. Lock in defaults, commit, move to next sub-stage.

### 3.1 Tectonics and base elevation

Design doc §1.9. Plate generation, boundary classification, base
elevation, sea level, discrete height levels.

PNG outputs: plate map (colored Voronoi), continuous elevation
(grayscale), discrete height levels (categorical), boundary types
(overlay).

Tuning targets: visible continental coherence, varied plate sizes,
no obvious Voronoi straight-line artifacts, sea level threshold
producing the requested land/water ratio.

### 3.2 Climate (temperature + moisture)

Design doc §1.10. Latitude-based temperature, advection-simulated
moisture, orographic precipitation.

PNG outputs: temperature field, moisture field, wind field
visualization, precipitation accumulation.

Tuning targets: visible rain shadows behind mountains, latitude
bands match expectations, no stripe artifacts from uniform wind, CFL
stability holds (no numerical blow-up).

This is the most numerically delicate stage. Expect parameter tuning
to take a while.

### 3.3 Hydrology

Design doc §1.11. Flow accumulation, sink resolution, lake detection,
river identification, floodplain marking, canyon carving.

PNG outputs: flow accumulation (log scale), rivers (binary mask
overlaid on elevation), lakes (overlay), floodplains (overlay),
canyons (overlay).

Tuning targets: rivers reach the sea or designated lakes (no orphan
flow), lake thresholds produce plausible counts, canyon carving
produces visible chokepoints without overpopulating.

**Blocker:** Resolve the cliff impassability question (design doc
§1.4, called out in §1.11). Canyon walls produce delta-2 height
transitions that need explicit traversability rules. Recommend
impassable for canyon walls; cleanest gameplay result.

### 3.4 Biomes

Design doc §1.12. Whittaker-style lookup from (temperature, moisture)
to biome, with hydrology overrides and optional smoothing.

PNG outputs: biome map (categorical), forest density (if implemented
as a separate feature pass).

Tuning targets: biome transitions look natural (not blocky), no
unhandled climate combinations producing undefined biomes, hydrology
overrides produce fertile strips along rivers in desert biomes.

The biome lookup table itself becomes a RON config file at this
stage (first real use of the RON serialization decision).

### 3.5 Resource placement

Design doc §1.13. Geological placement with plate mineral character,
biome filtering, clustering for luxuries, scattered for strategics.

PNG outputs: per-resource maps, combined resource overlay.

Tuning targets: rarity feels right (not too sparse, not too dense),
luxuries form characteristic continental monopolies, strategic
resources have realistic geographic distribution.

### 3.6 Starting position selection

Design doc §1.14. Resource-aware viability scoring, hard adequacy
floor, spacing constraints, optional audit step.

PNG outputs: viability heatmap, selected positions (overlay), audit
adjustments (if any).

Tuning targets: adequacy floor holds across many seeds, audit
triggers rarely (target <10%), spacing produces fair positions
without flattening geographic character.

This is the stage where the determinism scaffold gets a final
exercise — the full pipeline must produce byte-identical output
across runs.

### Gate: Stage 3 complete

- All six sub-stages implemented per their design doc sections.
- `mapdump` produces a full set of PNG outputs for any seed.
- Determinism tests pass for the full pipeline.
- A range of seeds produces visibly varied but consistently
  plausible maps.
- The design-doc-deferred decisions made during implementation
  (default map size, adequacy thresholds, biome table specifics,
  etc.) are committed and documented in the design doc.
- A new ADR is written for any cross-cutting decision that emerged
  during generation work (e.g., specific cliff traversability rules).

---

## Stage 3.5: Performance gate

**Goal:** Validate the smaller-tile design assumption (design doc §1.6)
before building game scaffolding that depends on it.

This is a hard gate, not a stage. The work is small but the result
gates everything downstream.

### 3.5.1 Stress test

Build a minimal Bevy app that:
- Generates a default-size map (working assumption 240×150 ≈ 36K tiles).
- Renders all tiles using instanced/batched rendering — one mesh,
  per-instance data for terrain type, height, etc.
- Supports camera panning across the full map.
- Reports FPS in an overlay.

### 3.5.2 Targets

- **60 FPS minimum at default zoom**, panning continuously across
  the map.
- **No frame-time spikes** when scrolling into unrendered regions
  (acceptable if there's a deliberate streaming/LOD system; not
  acceptable if it's a naive entity spawn).
- **Reasonable memory footprint** — back-of-envelope, on the order of
  hundreds of MB for the rendered scene, not gigabytes.

### 3.5.3 If the gate fails

Three possible responses, in order of preference:
1. Optimize the rendering approach (better instancing, GPU-side data,
   chunked rendering). Most likely outcome — naive Bevy rendering
   isn't optimized for 36K static tiles, but optimization is
   well-understood.
2. Revisit the default map size. 36K is a working assumption from
   the design doc, not a commitment. 25K may be enough.
3. **Revisit the smaller-tile design pillar.** Last resort, but
   honest — if the design can't be made to run, the design changes.
   This is exactly why the gate exists at this stage rather than
   after Stage 6.

Document the resolution in an ADR regardless of which path is taken.

### Gate: Stage 3.5 complete

- Stress test runs at 60+ FPS at default map size.
- ADR-NNNN records the rendering approach chosen and any deviations
  from the original design assumptions.

---

## Stage 4+: Placeholders

Stages 4 and beyond depend on design decisions that are still open
(design doc §4 roadmap, §5.3 deferred-but-foundational items).
These will be planned in detail when their prerequisite design work
is complete. The headings below sketch the territory without
committing to specifics.

### Stage 4 (placeholder): Game scaffolding

Bevy `App` setup, plugin architecture, the command layer
infrastructure (design doc §3 / pillar 0.4), basic world rendering
from generated `WorldData`. First time the `game` crate has real
content. Depends on Stage 3.5 passing.

### Stage 5 (placeholder): Turn structure and state machine

Turn phase state machine using Bevy's `States`. This is the design
doc's "next recommended architectural step" from the existing memory.
Depends on Stage 4 having a basic Bevy app to attach state machines
to.

### Stage 6 (placeholder): Tiles, terrain, basic interactivity

Tile entities spawned from `WorldData`. Terrain types, features,
resources visible. Click-to-select. Camera and input handling.
Depends on Stage 5.

### Stage 7+ (placeholder): Cities, units, mechanics

Design doc has substantial open topics here (§4 items 7–15). Plan
these stages when the design questions resolve — most importantly
the city growth dimension interactions, the production model, and
emergent vs declared regions.

The substantially-designed combat system (design doc §3.1–3.11)
is a separate, large body of implementation work that overlaps
with city/unit infrastructure. Its sequencing depends on what
city infrastructure looks like.

---

## Replanning protocol

This document is updated when:
- A gate is reached (mark the stage complete, possibly adjust the
  next stage's plan based on what was learned).
- A stage reveals a planning assumption was wrong (revise that stage
  or the next).
- Stage 3 completes (replan Stages 4+ with the design questions
  resolved).

This document is **not** updated for:
- Individual PR completions within a stage.
- Tactical work order within a stage (handled in the moment).
- Speculation about distant future stages.

If the plan and reality drift, the plan is wrong. Fix it.

---

## Status

**Current stage:** Stage 2 complete.

**Next action:** Begin stage 3.
