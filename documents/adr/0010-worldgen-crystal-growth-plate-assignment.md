# ADR-0010: Crystal-Growth Plate Assignment

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer  
**Supersedes:** ADR-0004 (partially — the fBm perturbation improvement is retained but the Voronoi assignment it improved is replaced)

## Context

ADR-0004 improved the boundary perturbation step from single-octave to
multi-octave fBm, concluding: "The BFS approach remains available if
plate size variety becomes a problem after tuning." Visual inspection
after that change showed that the improvement was real but structurally
insufficient.

The root problem is topological. Noise-based perturbation works by
displacing each tile's lookup position before the nearest-seed search.
This can shift and warp a Voronoi boundary into a curved or jagged line,
but it cannot fold the boundary back on itself. A Voronoi cell is always
convex in the displacement field; perturbation can deform but not invert
that convexity. No amplitude or frequency combination produces peninsulas,
interlocking fingers, or concave coastlines — the geometric features that
make real continental outlines recognisable.

Exhaustive sweeping of `boundary_perturbation_amplitude` (high values
warp boundaries further but create geometrically implausible crossings),
`boundary_perturbation_octaves` (adds fine-grain fractal detail at the
boundary edge but does not change macroscopic plate shape), and
`boundary_perturbation_frequency` (controls the scale of detail but not
the convexity constraint) confirmed that the failure mode is structural.
The Voronoi partition, however perturbed, produces plate outlines that
are visibly polygonal at the continental scale.

## Decision

Replace step 2 of the tectonics pipeline with crystal-growth (BFS-based)
plate assignment. The pipeline entry point (`tectonics.rs`) now calls
`growth::grow_plates_organic` instead of
`perturbation::assign_plates_perturbed`.

### Algorithm

Each plate maintains a *frontier*: the set of currently unassigned tiles
adjacent to tiles already belonging to that plate. The algorithm proceeds
in rounds:

1. Each plate's seed tile is claimed and its unassigned neighbours are
   added to its initial frontier.
2. Each round, every plate (visited in index order) picks one tile
   uniformly at random from its frontier and claims it. The claimed
   tile's unassigned neighbours are pushed onto the frontier. Stale
   frontier entries — tiles claimed by a competing plate since insertion
   — are discarded lazily on encounter.
3. Rounds continue until every tile is assigned. Termination is
   guaranteed because the hex grid is connected.

The result is plates that grow outward from their seeds like biological
colonies competing for territory. When two plates' frontiers meet they
interdigitate: the randomness of which plate claims a contested tile next
determines the exact boundary path. This produces:

- **Concave plate shapes** — a plate can grow around an obstacle and
  partially surround another plate's territory.
- **Interlocking fingers** — narrow peninsulas extending from one plate
  into another's region.
- **Varied plate sizes** — plates in open regions grow quickly while
  plates blocked by competitors stay small. The size distribution is
  naturally irregular, unlike Voronoi where the Poisson-disk spacing
  produces roughly uniform plate areas.

### What is unchanged

**Seed placement (step 1)** is unchanged. Seeds are still placed by
Poisson-disk sampling; the growth algorithm takes those seeds as its
starting tiles.

**Boundary classification (step 5)** is unchanged. The approach velocity
between two plates is computed as the dot product of their relative
motion with the unit vector from seed A to seed B. ADR-0004 raised this
as a risk: "A BFS plate map would require revisiting that assumption."
The concern does not materialise in practice because seeds remain as
point representations of each plate's identity and motion — the growth
algorithm changes which tiles belong to a plate, not the plate's motion
vector or its seed position. For the purpose of computing whether two
plates are converging, diverging, or sliding past each other, the
seed-to-seed vector remains a valid geometric approximation of the
boundary normal regardless of how irregular the actual boundary is.

**The perturbation module** (`perturbation.rs`, `voronoi.rs`) is retained
in the codebase. Their tests still pass; they serve as reference
implementations and are no longer called by the pipeline. Removing them
would delete test coverage and a working alternative implementation that
might be useful for future comparison.

### Determinism

`grow_plates_organic` takes a `StageRng` and makes all random frontier
selections through it. Identical seed and parameters → identical plate
map, satisfying the determinism requirement from ADR-0001.

## Consequences

**Positive:**

- Plate outlines acquire the concavity and interlocking structure of
  real continental margins without any noise parameter. The visual
  improvement over even highly-tuned Voronoi + perturbation is immediate
  and apparent at a glance.
- Plate size varies naturally. Large plates, medium plates, and small
  plates appear in the same world. Voronoi with Poisson-disk seeds
  produces roughly uniform plate areas (a consequence of the minimum-
  spacing radius); crystal-growth does not share this constraint.
- The algorithm is simple and its behaviour is visually intuitive: plates
  grow outward like colonies, compete at boundaries, and stop when no
  unassigned tiles remain.

**Negative:**

- `perturbation.rs` is now dead code in the live pipeline. It is retained
  for its tests and as a reference implementation but contributes compile
  weight without participating in generation.
- Crystal-growth plate boundaries are not smooth at any scale — they are
  one-tile-wide paths that meander based on RNG. The boundary elevation
  effects (Gaussian-spread modifiers) smooth this at the elevation level,
  but the underlying `plate_id` field has ragged boundaries. This is the
  correct visual character but is worth noting for any future stage that
  reads `plate_id` directly.
- Plate area is no longer predictable from the seed count and map size.
  A plate that happens to seed in a corner between two fast-growing
  neighbours may end up very small. At the default seed count of 17 on a
  240×150 map this produces geologically plausible variety; at very low
  seed counts (4–6) the size variance becomes extreme and some plates may
  consist of only a few tiles.

## Alternatives considered

**Continued fBm perturbation tuning** — rejected because the failure mode
is topological (convexity of Voronoi cells) rather than parametric. No
fBm configuration can make a Voronoi cell concave.

**Domain warping** — applying a second noise displacement to the
coordinates fed into the first. Produces highly distorted shapes but
at the cost of dramatic parameter sensitivity; mentioned in ADR-0004 as
available but not pursued. Rejected for the same structural reason as
perturbation tuning: the underlying partition remains Voronoi and
therefore convex in the pre-warp space.

**Weighted BFS (priority-queue ordering)** — instead of uniform random
frontier selection, weight tiles by their distance from the seed or by a
noise value. Would allow some control over plate "shape personality"
(compact vs. elongated). Deferred — the uniform random approach already
produces sufficient variety and adding a weight scheme adds a parameter
without a clear tuning target.

## References

- `crates/worldgen/src/stages/tectonics/growth.rs` — implementation
- `crates/worldgen/src/stages/tectonics/perturbation.rs` — retained but
  no longer called by the pipeline
- `crates/worldgen/src/stages/tectonics/voronoi.rs` — retained as
  reference implementation
- `crates/worldgen/src/stages/tectonics.rs:70` — the pipeline call site
- ADR-0004 — multi-octave fBm perturbation (the approach this supersedes)
- ADR-0001 — determinism requirement (context for RNG threading in growth)
- ADR-0002 — probabilistic plate type assignment (unchanged by this ADR)
