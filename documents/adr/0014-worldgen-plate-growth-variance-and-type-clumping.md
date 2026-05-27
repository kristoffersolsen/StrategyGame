# ADR-0014: Plate Growth Variance and Plate-Type Clumping

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer  
**Supersedes:** ADR-0010 (partially — extends organic plate growth;
the crystal-growth plate assignment decision still stands)

## Context

ADR-0010 replaced Voronoi plate assignment with organic crystal-growth
frontier expansion. That solved the most visible straight-line plate
boundary artifacts: plate outlines became irregular, concave, and
interlocking.

After visual tuning, a second artifact remained. Plate *shapes* varied,
but plate *sizes* were still too similar. The cause was the growth
loop itself: every plate received exactly one claim attempt per round.
Combined with Poisson-spaced seeds, this produced plates with similar
areas. The resulting height-level and elevation maps could read as
semi-tiled rather than as organic continents.

A related issue existed in plate-type assignment. Continental and
oceanic plate types were sampled independently per plate. Even with
irregular plate outlines, independent type sampling produced noisy
continent/ocean structure instead of larger continental clusters and
ocean-basin clusters.

The goal of this change is to add tunable variance and clustering
without changing default generation output. Default parameter values
must preserve the existing determinism reference hash.

## Decisions

### 1. Add per-plate growth-rate variance

`TectonicsParams` gains:

```text
plate_growth_rate_range: f32
```

The default is `1.0`, which is a strict no-op: every plate grows at the
original rate and no additional RNG is consumed.

When `plate_growth_rate_range > 1.0`, each plate samples a growth rate
uniformly from:

```text
[1 / plate_growth_rate_range, plate_growth_rate_range]
```

The organic growth loop tracks a floating-point credit per plate:

```text
credit[p] += growth_rate[p]
while credit[p] >= 1.0:
    claim one frontier tile
    credit[p] -= 1.0
```

A plate with growth rate `1.1` therefore claims roughly one extra tile
every ten rounds; a plate with growth rate `0.8` skips roughly one claim
every five rounds. This keeps the behavior easy to reason about and
preserves the existing frontier-claim mechanics.

The range is multiplicative and symmetric around `1.0` rather than
using `[1.0, max]`. That avoids shifting every non-default world toward
faster global growth and instead creates relative size variance between
plates.

Validation requires `plate_growth_rate_range >= 1.0` and finite.

### 2. Add plate-type clumping over the plate adjacency graph

`TectonicsParams` gains:

```text
clumping_index: f32
```

The default is `0.0`, which is a strict no-op: plate types remain
independently sampled and no additional RNG is consumed.

When `clumping_index > 0.0`, plate types are smoothed after initial
assignment and before boundary classification/elevation generation.
The implementation builds the plate adjacency graph from the grown
`plate_id` field, then runs a small fixed number of deterministic
local-majority passes. On each pass, each plate compares the
continental/oceanic types of adjacent plates:

- if there is no majority, the plate keeps its current type;
- if the majority matches the current type, the plate keeps it;
- if the majority differs, the plate adopts the majority type with
  probability `clumping_index`.

Validation requires `clumping_index` to be finite and in `[0.0, 1.0]`.

### 3. Defaults preserve existing output

Both new defaults are no-op values:

```text
plate_growth_rate_range = 1.0
clumping_index = 0.0
```

The code checks those no-op values before sampling new random values.
This preserves the prior RNG stream and keeps the existing determinism
reference hash unchanged.

### 4. Expose both controls through tuning tools

Both parameters are included in:

- `tools/maptest/presets/default.ron`;
- `tools/maptest/presets/test.ron`;
- `tools/maptest/presets/more_land_levels.ron`;
- `tools/maptest/src/sweep.rs`;
- `scripts/mapgen_sweep.sh` as commented sweep candidates.

Recommended initial sweeps:

```text
plate_growth_rate_range: 1.0..2.0
clumping_index: 0.0..1.0
```

## Consequences

**Positive:**

- Plate-size variance becomes tunable without discarding the organic
  crystal-growth algorithm from ADR-0010.
- Continents and ocean basins can form from clusters of same-type
  plates instead of depending on independent per-plate sampling.
- Default behavior remains byte-identical, which keeps existing
  tuning baselines and reference hashes valid.
- The new controls are compatible with existing visual sweep tooling,
  so they can be tuned using plate maps, elevation maps, height levels,
  and hydrology overlays.

**Negative:**

- High `plate_growth_rate_range` can create very small plates and long
  plate boundaries. This may overproduce mountains, hills, and rugged
  terrain unless tuned with the boundary/elevation parameters.
- High `clumping_index` can collapse plate-type diversity into large
  same-type regions. That may produce overly simple continent/ocean
  structure or reduce interesting subduction/coastal-boundary variety.
- The clumping pass is a local smoothing process, not a global
  constraint solver. It does not guarantee an exact continental/oceanic
  plate count after smoothing. Sea-level quantile assignment still
  enforces final height-level proportions.

## Alternatives considered

**Per-tile growth rate** — assign growth rates to individual tiles and
let fast tiles expand faster. Rejected because the intended artifact is
plate-size uniformity, not local boundary roughness. Per-tile rates
would add noise to already-irregular boundaries and make the result
harder to reason about. Per-plate rates target the actual problem.

**Sample plate target sizes up front** — draw target area weights for
plates and force growth to match those weights. Rejected because it
would turn the current local frontier-growth algorithm into a global
allocation problem. It would also create awkward behavior when a plate
exhausts plausible frontier space before reaching its target size.
Credit-based growth is simpler and preserves the existing mechanics.

**Use `[1.0, max]` for growth rates** — all plates grow at least as fast
as the old rate, with some growing faster. Rejected because the absolute
speed of global growth is not meaningful; only relative growth speed
between plates matters. A symmetric multiplicative range gives size
variance without biasing the whole process.

**Cluster plate types before growth** — decide continental/oceanic
clusters at seed placement time, then grow plates. Rejected because the
actual adjacency graph is only known after growth. Post-growth clumping
uses the real plate graph and therefore aligns type clusters with the
territories that boundary classification and elevation generation will
actually consume.

**Noise-field plate typing** — sample a low-frequency noise field at
each plate seed and assign continental/oceanic type by threshold. This
would naturally clump nearby seeds. Rejected for now because the map is
cylindrical and seed spacing is already stochastic; adding another
noise field would require new frequency/amplitude tuning and could
couple plate-type clumping to map scale in subtle ways. The adjacency
graph method is scale-independent and easier to tune.

## Revisit conditions

- If plate-size variance produces too many tiny plates at useful
  settings, replace uniform growth-rate sampling with a narrower
  distribution around `1.0` or add a soft lower area bound.
- If clumping commonly destroys continental/oceanic balance, add a
  post-clump correction pass that preserves approximate type counts
  while still favoring same-type adjacency.
- If continent shapes still read as tiled after tuning these two
  parameters, revisit seed placement itself. The next likely lever is
  adding clustered or hierarchical plate seeds rather than changing
  growth mechanics further.

## References

- ADR-0010 — crystal-growth plate assignment.
- `crates/worldgen/src/stages/tectonics/growth.rs` — per-plate growth
  rate credit implementation.
- `crates/worldgen/src/stages/tectonics/plates.rs` — plate-type
  clumping implementation.
- `crates/worldgen/src/stages/tectonics/elevation.rs` — downstream
  elevation generation that consumes clumped plate types.
- `crates/worldgen/src/params/tectonics.rs` — tuning parameters and
  validation.
- `tools/maptest/src/sweep.rs` — sweep exposure for both controls.
