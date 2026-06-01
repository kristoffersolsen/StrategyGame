# Elevation Transition Rendering — Methodology Summary

*Captured for later implementation. Out of scope for the current round.
Elevation differences are mechanically meaningful (not cosmetic), which
drives the core/view split noted below.*

## Principle

Discrete elevation tiers are the simulation's truth. The renderer never
reintroduces continuous height as state. Transition geometry between tiers
is *derived from the discrete tiers*, not from any leftover continuous
height field — so the visual is always a faithful view of game truth, never
a second source of it. This is the trust-tier discipline applied to terrain
geometry: the mesh is downstream of discrete state.

## The lattice model (points are primary, tiles are a view)

The naive approach — each tile samples height at its corners/edges/center
and builds its own mesh — produces cracks and z-fighting, because a point
shared by several tiles gets evaluated independently by each and the results
disagree. The fix is to invert the model:

- **Corners are the primary data.** Each corner is evaluated **once**, reads
  the (up to three) tiers meeting there, and all adjacent tiles read that one
  result. Watertightness holds *by construction* — there is no second
  computation to disagree with.
- **A tile is a stitching rule**, not a geometry owner: "connect these six
  shared corners and this center into a surface."
- The points form a dependency order, not a flat set of equal evaluations:
  **corners decide** (height, and feed edge classification) → **edges
  classify** (slope / cliff / flat, from their two tiers) → **centers and
  edge-midpoints are derived** (center = the tile's tier height; midpoints =
  tessellation detail positioned from the corner/edge decisions). Center <
  edge < corner in how much each decides.

## Two-pass pipeline

1. **Classify each edge** from the discrete tier delta of its two tiles:
   delta 0 = flats meet; small delta + passable = slope/ramp; large delta or
   impassable = cliff (sharp lip, near-vertical face). **This pass is
   core-owned**, because passability is mechanical game state — the
   simulation decides "this edge is a cliff (impassable)"; the renderer does
   not decide it from appearance.
2. **Generate mesh** (view) from the classified edges and the once-evaluated
   corner heights. Tile flats sit at their tier height; transition geometry
   bridges the edges per their classification.

## The hard part: the three-tier corner

On a hex grid every interior corner is a three-way meeting, so a corner where
**three different tiers** meet is common, not an edge case. It has no single
obvious geometry (several defensible resolutions exist: central interpolated
triangle, stepped, dominant-tier-wins) and must stay watertight against all
surrounding two-tier edges. **This is the one genuinely fiddly piece** — a
geometry-design decision more than a coding problem.

Implementation order when this is built: **prototype the three-tier corner
first**, across all tier-difference combinations, before building the
surrounding machinery. If the corner resolution is solid the rest assembles
around it; building the easy two-tile edges first risks redoing them to fit
whatever the corner demands.

## Performance

Build-time cost, not frame-time. The lattice and transition mesh are baked
into chunk meshes once at world setup and drawn as static geometry thereafter.
Elevation rarely changes; when a tier does change (terraforming), the blast
radius is the tile plus its ring of neighbors' shared boundaries — a handful
of corners/edges re-evaluated and one or two chunks re-baked. Never a
per-frame concern.

## Open decisions (resolve when implementing)

- Three-tier corner resolution geometry — explicit choice among the options
  above.
- Exact tier-delta thresholds for slope vs. cliff, and how they map to the
  core's passability classification.
- Tessellation density of transition surfaces (edge-midpoint vertex count).

## Architecture note

When built, record in `documents/architecture/architecture.md`: *terrain
transition lattice is view-derived from discrete tiers; edge passability
classification is core-owned.* The lattice is mostly presentation with a
single well-defined tendril into game state (the edge classification), so it
can be rebuilt entirely from discrete state plus the core's classifications
and never becomes an independent source of truth.
