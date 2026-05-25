# ADR-0002: Probabilistic Plate Type Assignment

**Status:** Accepted  
**Date:** 2026-05-24  
**Deciders:** Kristoffer

## Context

Each tectonic plate must be assigned a type — continental or oceanic —
before base elevation is computed. Continental plates sit at positive
base elevation; oceanic plates at negative. The ratio of continental to
oceanic plate area loosely controls how much of the map will be land
before boundary effects and noise are applied.

The player-exposed `land_fraction` parameter (default 0.4) expresses the
*desired* land ratio. There are two ways to use it during plate type
assignment:

**Probabilistic assignment:** assign each plate continental with
probability `land_fraction` and oceanic with probability
`1 - land_fraction`, independently per plate. Simple and fast, but
the achieved continental *area* fraction varies by seed because plates
differ in size. A seed that happens to produce large plates in
continental positions could yield significantly more land than the
target.

**Area-targeted assignment:** compute each plate's approximate tile
count from the Voronoi partition, then greedily assign types so the
total continental tile count is as close to `land_fraction × total_tiles`
as possible. More reliable across seeds, but requires sorting plates by
size and iterating to hit the target.

The key context is that **step 9 (quantile thresholding) enforces the
exact land ratio regardless of plate type assignment**. The sea level
threshold is computed as the elevation value at the `(1 - land_fraction)`
quantile of the continuous elevation field, guaranteeing the correct
land/water split in the final output. Plate type assignment only
determines the *shape* of the elevation distribution — whether
continental regions feel geologically elevated and oceanic regions feel
depressed — not the ultimate land fraction.

## Decision

Use **probabilistic assignment**. Each plate is assigned continental
with probability `land_fraction`, independently.

The exact land fraction in the final map is enforced by quantile
thresholding at step 9, not by plate type assignment. The role of plate
type is to produce a bimodal elevation distribution (continental hump
above zero, oceanic basin below) that makes geological features like
mountain chains and trenches coherent. Probabilistic assignment
produces this distribution adequately, and any seed-to-seed variation
in the raw distribution is corrected downstream.

## Consequences

**Positive:**
- Simple to implement: one `rng.random_bool(land_fraction)` call per
  plate.
- No dependency on plate area computation at this step; plate sizes
  are not yet computed when types are assigned.
- Variation in the raw continental area fraction across seeds adds
  geological variety — some seeds will have more large continents,
  others more fragmented landmasses, even before boundary effects.

**Negative:**
- On seeds that produce very unequal plate sizes, the raw elevation
  distribution before thresholding may be heavily skewed. This could
  affect how much "work" the boundary effects and noise overlays need
  to do to produce plausible topography, and may interact poorly with
  the Gaussian falloff tuning in step 7.
- If the skew is severe enough, step 9's thresholding may produce a
  land mass that is geographically implausible even if the ratio is
  numerically correct (e.g., a single giant continent with no internal
  variation).

## Alternatives considered

**Area-targeted assignment:** sort plates by tile count after Voronoi
partition, then assign types greedily to hit `land_fraction × total_tiles`
as closely as possible. This would reduce seed-to-seed variation in the
raw elevation distribution and make the elevation field more predictable
before thresholding. Rejected for now because the added complexity is
not justified until visual results from step 9 show the probabilistic
approach producing implausible maps. If that happens, this ADR should
be superseded.

## Revisit condition

If visual inspection across a range of seeds (after step 9 is
implemented) shows maps where the land distribution is geographically
incoherent — for example, the target land fraction is met but all land
is clustered in one corner — switch to area-targeted assignment and
write ADR-0003 superseding this one.

## References

- Design doc §1.9 — tectonics algorithm, plate type assignment
- `crates/worldgen/src/stages/tectonics/` — implementation
- `AdvancedParams::land_fraction` — the player-exposed parameter