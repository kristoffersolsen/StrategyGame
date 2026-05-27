# ADR-0011: Hydrology Algorithm Choices (Stage 3.3)

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

Stage 3.3 implements the hydrology pipeline: flow direction, sink
resolution, flow accumulation, river identification, floodplain marking,
and canyon carving. Each of these steps has non-obvious algorithm choices
with meaningful consequences for output quality, determinism, and
downstream stage correctness. This ADR records the choices made and the
alternatives considered.

The hydrology stage consumes `TectonicData` (elevation, height levels)
and `ClimateData` (precipitation) from the prior stages and produces
`HydrologyData` containing the river network, lake tiles, floodplain
features, and canyon features.

## Decisions

### 1. Flow direction: D6 single-downstream

Each land tile is assigned exactly one downstream neighbor — the
adjacent tile with the lowest elevation — using the six hex neighbors
(D6). Ties are broken deterministically by `Direction` enum ordinal
(fixed order: East, SouthEast, SouthWest, West, NorthWest, NorthEast).
Ocean and coast tiles are terminals; flow reaching them exits the
system. Lake tiles (after sink resolution) are also terminals; flow
entering a lake exits via the lake's spill point.

### 2. Sink resolution: hybrid Planchon-Darboux

Sinks (land tiles with no downhill neighbor) are resolved using a
hybrid strategy rather than filling all sinks uniformly:

- **Small basins** (area below `lake_basin_area_threshold`): filled
  using Planchon-Darboux elevation raising. The basin floor is raised
  to the spill point elevation, producing flat drainage that routes
  flow onward. No lake is created.
- **Large basins** (area at or above threshold): all tiles below the
  spill point elevation are marked as lake. Flow enters the lake via
  rivers and exits through the spill point tile.

The Planchon-Darboux fill uses a min-heap priority queue ordered by
elevation. Iteration over the open set uses a fixed `(r, q)`
lexicographic tie-break on equal-elevation tiles for determinism.

### 3. Flow accumulation: precipitation-weighted topological sort

After sink resolution, every tile has a valid downstream path to ocean
or lake. Flow accumulation runs as a single O(N) pass over tiles sorted
in descending elevation order (ties broken by `(r, q)` coordinate
order). Each tile seeds its own `ClimateData::precipitation` value into
its accumulator, then adds its accumulator to its downstream neighbor's.
One pass suffices because the elevation sort is a valid topological
ordering of the acyclic flow graph.

### 4. Canyon carving: post-hoc stream power, no flow recomputation

Canyon carving runs after flow accumulation and river identification.
For each river tile, the product `flow_accumulation × local_slope` is
computed (where local slope is the normalized elevation drop to the
downstream neighbor). Tiles where this product exceeds
`canyon_flow_slope_threshold` are carved: their `height_level` is
lowered by `canyon_depth` levels, and the two neighbors perpendicular
to the flow direction are marked as canyon wall features.

The flow network is **not recomputed** after carving. Canyon formation
occurs where large rivers cross steep terrain — the same locations where
flow was already high — so the pre-carve flow direction is stable.
Recomputing would change nothing for the canyon tiles themselves and
would add O(N log N) cost for no gameplay benefit.

Canyon walls carry an impassability flag (see ADR-0012 and design doc
§1.4): height delta ≥ 2 between a canyon floor tile and its wall tile
is impassable by default.

### 5. Determinism

All passes use fixed `(r, q)` lexicographic iteration order. No
parallelism in inner loops. The priority queue in Planchon-Darboux uses
a secondary key of `(r, q)` to break elevation ties. These constraints
match the pattern established in the climate stage (ADR-0004).

## Consequences

**Positive:**

- D6 single-downstream produces discrete, identifiable river channels
  suitable for the tile-property river model (design doc §1.5). Each
  river tile has unambiguous identity and flow direction, which is
  needed for floodplain detection and canyon carving.
- Hybrid sink resolution produces both plausible flat plains (small
  filled depressions) and named lakes (large basins), matching
  real-world geography without requiring a separate lake-placement pass.
- Precipitation-weighted accumulation connects the climate and hydrology
  stages directly: wet regions produce larger rivers than dry regions,
  and Nile-pattern rivers (fed by wet highland precipitation flowing
  through arid lowlands) emerge naturally.
- Post-hoc canyon carving is O(N) and geologically plausible: canyons
  form at high `flow × slope` locations, matching the stream power law.

**Negative:**

- D6 single-downstream produces geometric artifacts on flat terrain:
  rivers tend to follow straight lines along the noise ridge axes rather
  than meandering naturally. Acceptable for v1; the orographic noise on
  the elevation field breaks the worst cases.
- Hybrid sink resolution requires tuning `lake_basin_area_threshold`
  carefully. Too low produces lakes everywhere; too high produces none.
  The sweep infrastructure handles this.
- Not recomputing flow after canyon carving is a known approximation.
  In theory, lowering a canyon tile's elevation could change which
  direction a neighboring tile drains. In practice this does not occur
  because canyons are carved along existing high-flow river channels,
  which are already the lowest paths through the terrain.

## Alternatives considered

**D-infinity multi-flow direction** — distributes flow across all
downhill neighbors proportionally to slope angle. Produces smoother,
more natural-looking drainage basins and eliminates the straight-river
artifact. Rejected because it produces fractional flow values that do
not map cleanly to the discrete tile-property river model (design doc
§1.5) — a tile either is or is not a river, and D-infinity makes every
tile a partial river. The downstream rendering and gameplay model assumes
discrete rivers.

**Pure Planchon-Darboux fill (no lake detection)** — fill all sinks
regardless of basin size. Simpler to implement (no threshold, no lake
marking). Rejected because it eliminates inland lakes from the map
entirely, which are geographically important (Great Lakes, Caspian Sea
analog) and are explicitly called out in the design doc as desired
features of the hydrology output.

**Skip sink resolution entirely** — leave sinks as-is, accept orphan
flow chains. Rejected because rivers that fail to reach the sea or a
lake produce broken visual output (rivers ending mid-continent) and
violate the tuning target that every flow path must terminate at ocean
or lake.

**Iterative erosion (stream power law simulation)** — run multiple
rounds of flow accumulation and elevation lowering, converging toward
an equilibrium. Produces highly realistic canyon and valley
morphology. Rejected because the iteration count required for
convergence on a 36K-tile map is prohibitive, and the output is
sensitive to initial conditions in ways that make parameter tuning
difficult. Post-hoc carving achieves the same visual goal (canyons at
high `flow × slope` locations) without iteration.

**Recompute flow after canyon carving** — run a second flow accumulation
pass using the carved elevation field. Rejected as unnecessary: canyon
tiles are carved because they already have high flow and high slope, so
the pre-carve flow direction is stable. The O(N log N) re-sort cost is
not justified by the marginal accuracy gain.

## Revisit conditions

- If D6 straight-river artifacts become visually prominent enough to
  affect map quality after biome rendering lands (Stage 3.4), revisit
  adding a small elevation noise perturbation pass before flow direction
  computation to break the geometric regularity.
- If lake count is consistently too low or too high across the seed
  range after the initial `lake_basin_area_threshold` sweep, consider
  replacing the fixed-tile-count threshold with a proportional threshold
  (e.g., fraction of map area) to make the parameter seed-invariant.

## References

- Design doc §1.11 — hydrology stage algorithm design
- Design doc §1.5 — rivers as tile property, not edge-based
- ADR-0004 — climate stage determinism strategy (iteration order
  convention this stage follows)
- ADR-0012 — hydrology elevation ownership and canyon wall
  impassability (companion ADR)
- `crates/worldgen/src/stages/hydrology.rs` — implementation entry
  point
