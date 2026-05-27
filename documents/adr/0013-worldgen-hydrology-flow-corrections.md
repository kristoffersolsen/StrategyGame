# ADR-0013: Hydrology Flow Direction Corrections

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer  
**Supersedes:** ADR-0011 (partially — replaces the flow direction and
lake routing decisions; all other decisions in ADR-0011 stand)

## Context

Two defects were identified after the initial Stage 3.3 implementation:

**Defect 1 — Lakes are pure sinks.** The spill point computed during
basin resolution was stored only temporarily and discarded after lake
tiles were marked. Flow accumulation treated all lake tiles as
terminals, so no water exited through the spill point. Rivers feeding a
lake produced zero outflow, and any terrain downstream of a lake had no
river network.

**Defect 2 — River channel clumping.** Adjacent headwater tiles with
equally-valid downhill neighbors independently picked the same
downstream tile, producing parallel river channels that ran side by side
for several tiles before merging. This was a model defect: the parallel
tile's precipitation entered the main channel one tile later than it
should, producing incorrect flow accumulation values for floodplain
detection and biome moisture on the parallel tile itself.

ADR-0011 recorded the original decisions. This ADR records the
corrections and supersedes the relevant portions.

## Decisions

### 1. Lake outlets stored and used during accumulation

The spill point's `outside` coordinate is now stored as a
`Field<Option<Coord>>` named `lake_outlet` in `ResolvedBasins` and
`HydrologyData`. Every tile that is marked as lake records the outlet
coordinate for its basin. Basins with no outside neighbor (fully
enclosed map edge cases) record `None` and remain true sinks.

During flow accumulation, lake tiles with a non-`None` outlet push
their accumulated flow into the outlet tile instead of stopping. The
outlet tile is a normal land tile at or below spill elevation; it drains
normally and requires no special handling beyond receiving the inflow.

Flow direction for lake tiles remains `None`. Direction is meaningless
inside a lake body; the outlet is an explicit coordinate, not a
direction. The accumulation loop handles the two cases — directional
land tiles and outlet-bearing lake tiles — with a single conditional.

### 2. Flow direction computed via elevation-sorted pass with channel-bias tiebreak

`build_flow_direction` is changed from a `Field::from_fn` (arbitrary
evaluation order) to an elevation-sorted pass (descending elevation,
`(r, q)` lexicographic tiebreak for determinism).

During neighbor selection for each tile, the steepest-descent neighbor
is found as before. If multiple neighbors share that minimum elevation
exactly (a genuine tie from the noise field), the tiebreak now checks
whether any tied neighbor already has a flow direction assigned —
meaning it is an active channel tile processed earlier in the sorted
order. If so, that neighbor is preferred over a non-channel neighbor at
the same elevation. If multiple tied neighbors are active channels, the
one with lower elevation is preferred; if still tied, the `Direction`
enum ordinal is the final tiebreak.

The bias applies only within exact floating-point ties at the minimum
elevation. A channel neighbor at any elevation higher than the
steepest-descent neighbor is never preferred — water does not flow
uphill. The fix is conservative by design.

Because tiles are processed highest-first, any neighbor that has
already been assigned a direction has received that assignment from its
own uphill sources. It is a genuinely active channel, not an artifact
of evaluation order. The bias is therefore physically grounded:
the current tile's water joins the existing channel one tile sooner,
which is what happens geographically when two drainage paths of equal
slope share the same lowest point.

This is an order-dependent algorithm. The evaluation order is
load-bearing. The `flow_direction.rs` module-level doc records this
constraint explicitly.

## Consequences

**Positive:**

- Lakes now produce outflow rivers at their spill points. River
  networks downstream of lakes are correctly populated. The Nile-pattern
  case (wet highland lake draining through an arid lowland) works
  correctly.
- Parallel river clumping is eliminated at the model level, not papered
  over at the rendering level. Flow accumulation values on consolidated
  channel tiles are correct, which means floodplain detection and biome
  moisture reads correct values.
- The sorted evaluation order for flow direction is identical to the
  sort already needed for accumulation. The two passes can share a
  single sorted tile vector, avoiding a redundant sort.

**Negative:**

- `build_flow_direction` is no longer a pure function over a single
  tile; it depends on the results of previously-processed tiles. This
  makes it harder to unit-test in isolation. Tests must either construct
  explicit elevation fields that exercise the tiebreak, or test the
  full sorted-pass output rather than per-tile behavior.
- The channel bias applies only to exact floating-point ties. On
  terrain where the noise field produces near-ties but not exact ties,
  some parallel-flow artifacts may remain. This is acceptable for v1;
  the fix eliminates the systematic cases.
- The `lake_outlet` field adds one `Option<Coord>` per tile to
  `HydrologyData` for the duration of the debug period. `Coord` is two
  `i32`s; at 36K tiles this is roughly 288 KB. Negligible.

## Alternatives considered

**Post-classification suppression pass** — after classifying river
tiers, remove the river label from any tile that has a higher-flow
river neighbor. Rejected because it leaves the flow accumulation values
wrong. A suppressed tile still has its parallel-channel accumulation
value, which floodplain detection and biome moisture would read
incorrectly. Suppression is a rendering fix, not a model fix.

**Post-direction correction pass** — after computing D6 flow directions
independently, do a second pass redirecting parallel-flow pairs. One
tile in each parallel pair is redirected to drain into the other rather
than bypassing it to the shared downstream. Rejected because the
correction can chain: redirecting one pair may create a new parallel
pair with a third tile, requiring iteration to convergence. The
sorted-pass approach eliminates the problem on the first pass without
iteration.

**River-burning trace** — select tiles from a priority queue and trace
each one to a sink before selecting the next, removing visited tiles
from the queue as the channel is established. Produces explicit river
paths and naturally consolidates channels. Rejected because the result
depends on which tile is selected first from the queue, requiring
careful ordering to maintain determinism. The sorted-pass approach
achieves the same channel-consolidation effect with a simpler and more
obviously deterministic algorithm.

**D-infinity multi-flow** — distribute flow across all downhill
neighbors proportionally to slope, eliminating the single-channel
constraint. Rejected as in ADR-0011: produces fractional flow values
incompatible with the discrete tile-property river model. The channel
bias is a targeted fix that preserves the D6 model's discrete river
identity while correcting its most visible artifact.

**Outlet-only lake routing (no direction change)** — an alternative
lake fix would have assigned lake tiles a flow direction pointing toward
the outlet, making them behave like ordinary directed tiles during
accumulation. Rejected because flow direction inside a lake body is
physically meaningless — lakes do not have a single flow direction.
Storing the outlet as an explicit coordinate and handling it separately
in accumulation is more honest about what a lake actually is.

## References

- ADR-0011 — original hydrology algorithm decisions (partially
  superseded by this ADR)
- `documents/planning/hydrology-fix.md` — defect analysis and fix
  specification
- `crates/worldgen/src/stages/hydrology/flow_direction.rs` —
  elevation-sorted pass implementation
- `crates/worldgen/src/stages/hydrology/basin.rs` — lake outlet storage
- `crates/worldgen/src/stages/hydrology/accumulation.rs` — lake outlet
  routing during accumulation
- `crates/worldgen/src/world_data.rs` — `lake_outlet` field in
  `HydrologyData`
