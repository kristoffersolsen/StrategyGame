# Hydrology Fix: Lake Outlets and River Channel Bias

Two defects identified after initial implementation. This document
describes each defect, its root cause, and the precise fix.

---

## Fix 1: Lake outlets

### Defect

All lakes act as flow sinks. Water enters a lake via rivers and stops;
no flow exits through the spill point. This means any river system that
drains into a lake produces no outflow river, and flow accumulation
downstream of a lake is zero.

### Root cause

`find_spill_point` in `basin.rs` computes the correct spill point for
large basins, but the result is discarded after the lake is marked.
`build_flow_direction` in `flow_direction.rs` treats all lake tiles as
terminals (`is_lake[coord]` → `None`), and `accumulate_flow` in
`accumulation.rs` stops propagating at any tile with no flow direction.
The spill point is never stored and never consulted during accumulation.

### Fix

**1. Store the lake outlet in `HydrologyData`.**

Add a `Field<Option<Coord>>` named `lake_outlet` to `ResolvedBasins`
(and subsequently `HydrologyData`). For each large basin, record the
spill point's `outside` coordinate as the outlet for every tile in that
basin's lake. For basins with no spill point (no outside neighbor at
all — an edge case for fully enclosed maps), the outlet remains `None`
and the lake truly is a sink.

In `basin.rs`, `mark_lake_below_spill` gains a `lake_outlet` parameter
and writes `Some(spill.outside)` for every tile it marks as lake.
`mark_lake` (the no-spill-point path) writes `None`.

**2. Route flow through the outlet during accumulation.**

In `accumulate_flow` in `accumulation.rs`, change the per-tile loop
body. Currently, tiles with `flow_direction[coord] == None` are skipped.
After the fix, add a secondary check: if `is_lake[coord]` is true and
`lake_outlet[coord]` is `Some(outlet)`, push the tile's accumulated
flow into `outlet` instead of stopping.

The outlet tile is a normal land tile (the spill-point neighbor outside
the basin). It receives the lake's accumulated inflow and drains
normally from there. No changes needed to `build_flow_direction` — lake
tiles correctly remain directionless, because direction is meaningless
inside a lake body; the outlet is an explicit coord, not a direction.

**3. Accumulation sort order.**

The topological sort in `accumulate_flow` sorts by descending elevation.
Lake tiles were filled to their spill elevation during sink resolution,
so they sort correctly relative to their outlet — the outlet tile is at
or below spill elevation and will be processed after the lake tiles.
No change needed to the sort.

### Affected files

- `crates/worldgen/src/stages/hydrology/basin.rs` — store outlet coord
  in `ResolvedBasins`; pass through `mark_lake_below_spill`.
- `crates/worldgen/src/stages/hydrology/accumulation.rs` — route lake
  flow to outlet instead of stopping.
- `crates/worldgen/src/world_data.rs` — add `lake_outlet: Field<Option<Coord>>`
  to `HydrologyData`.

---

## Fix 2: River channel consolidation via elevation-sorted flow direction

### Defect

Rivers clump: adjacent tiles both independently pick the same downstream
neighbor as their steepest descent, producing parallel river tiles that
run side by side before merging. This is a model defect, not a rendering
artifact — the flow accumulation values are wrong because the parallel
tile's precipitation contributes to the shared downstream tile one step
later than it should, and floodplain detection on the parallel tile reads
incorrect flow values.

### Root cause

`build_flow_direction` uses `Field::from_fn`, which evaluates each tile
independently with no knowledge of what neighboring tiles have been
assigned. The evaluation order is arbitrary, so there is no opportunity
to prefer draining into an already-established channel.

### Fix

Replace the `Field::from_fn` in `build_flow_direction` with an
elevation-sorted pass that processes tiles highest-first and applies a
channel-bias tiebreak during neighbor selection.

**Algorithm:**

1. Collect all land tiles (height level > 1, not lake) into a `Vec<Coord>`.
2. Sort descending by elevation, with `(r, q)` lexicographic tiebreak
   for determinism.
3. Allocate a mutable `Field<Option<Direction>>` initialized to `None`.
4. For each tile in sorted order:
   a. Find the steepest-descent neighbor (lowest elevation strictly
      below current tile). Call its elevation `best_elevation`.
   b. Among all neighbors whose elevation equals `best_elevation`
      (within floating-point exact equality — these are ties from the
      noise field), check whether any already has a flow direction
      assigned (i.e. is an active channel tile).
   c. If a tied neighbor has an assigned flow direction, prefer it.
      If multiple tied neighbors have assigned flow directions, prefer
      the one with the lower elevation; if still tied, use the
      `Direction` enum ordinal as the final tiebreak.
   d. If no tied neighbor has an assigned flow direction, fall back to
      the `Direction` ordinal tiebreak as before.
   e. Write the chosen direction into the field.

**Why highest-first ordering works:**

When tile T is evaluated, all tiles with higher elevation have already
been processed. A neighbor that has been assigned a direction has
received that assignment from its own uphill neighbors — it is
genuinely an active drainage channel, not an artifact of arbitrary
order. Preferring it means T joins an existing channel rather than
running parallel to it. The bias is physically grounded: T's water
reaches the main channel one tile sooner, which is correct.

**Tolerance note:**

The tiebreak applies only to exact floating-point ties among the
minimum-elevation neighbors. It does not apply when a channel neighbor
is at a higher elevation than the steepest-descent neighbor — water
never flows uphill. This keeps the fix conservative: it only affects
tiles where the terrain genuinely offers multiple equally-valid
downhill directions.

**Shared sort with accumulation:**

The sorted tile order computed here is identical to the sort used in
`accumulate_flow`. Consider computing it once in the hydrology entry
point (`hydrology.rs`) and passing it to both `build_flow_direction`
and `accumulate_flow` to avoid sorting twice.

### New ordering constraint

Flow direction assignment now depends on tile evaluation order. This
is a new constraint on `build_flow_direction` that did not exist in the
original implementation. It must be documented clearly:

- The `//!` doc on `flow_direction.rs` must state that the function
  processes tiles in descending elevation order and that this order is
  load-bearing for channel consolidation.
- ADR-0013 records the decision and the reasoning.

### Affected files

- `crates/worldgen/src/stages/hydrology/flow_direction.rs` — replace
  `Field::from_fn` with elevation-sorted pass; add channel-bias
  tiebreak.
- `crates/worldgen/src/stages/hydrology/accumulation.rs` — optionally
  accept a pre-sorted tile order to avoid re-sorting.
- `crates/worldgen/src/stages/hydrology.rs` — optionally compute the
  sorted order once and pass it to both steps.

---

## Implementation order

Fix 1 (lake outlets) first. It is self-contained, does not touch flow
direction, and makes the lake-outlet field available for visual
verification before Fix 2 changes the flow graph.

Fix 2 (channel bias) second. Once Fix 1 is verified, replace
`build_flow_direction` with the sorted pass. Re-run the full pipeline
and verify that parallel river artifacts are eliminated and lake outlets
still produce correct outflow rivers.

Both fixes are independently PR-able.
