# ADR-0007: Donor-Cell Upwind Advection on the Hex Grid

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

Moisture transport is governed by the advection-source-sink equation:

```
∂M/∂t + ∇·(vM) = S − P
```

where `M` is moisture, `v` is the wind velocity field, `S` is the
evaporation source, and `P` is precipitation. Discretising this on a
hex grid requires choosing an advection scheme, a coordinate frame for
wind projection, a stability criterion, a precipitation coupling
strategy, and an orographic precipitation model.

These choices interact: the stability criterion constrains the wind
amplitude parameter, the coordinate frame determines what "outflow
fraction" means, and the precipitation coupling affects whether
iteration order influences the result.

## Decision

### Advection scheme: donor-cell upwind

Each tile distributes a fraction of its moisture to its six hex
neighbours proportional to the dot product of the wind vector with the
unit vector in each neighbour direction. Only positive dot products
(outflow directions) contribute. The tile retains the complement.

```
p_k = max(0, dot(wind, unit_k))      // outflow fraction toward neighbour k
W   = Σ p_k                           // total outflow fraction
retained = M × (1 − W)               // moisture kept by the tile
sent_k   = M × p_k                   // moisture sent to neighbour k
```

This is first-order upwind (donor-cell) discretisation. It is
diffusive but unconditionally positive (no negative moisture values
arise from the advection step alone) and straightforward to implement
in a single pass over the field.

### Coordinate frame: Cartesian hex space

Wind vectors are expressed in a 2D Cartesian frame derived from the
flat-top hex axial-to-Cartesian mapping:

```
x = 3/2 · q,   y = √3/2 · q + √3 · r
```

The six neighbour unit vectors are computed by applying this mapping
to each of the six axial direction offsets and normalising. The
resulting unit vectors are spaced exactly 60° apart:

```
East:      (√3/2,  1/2)    West:      (−√3/2, −1/2)
NorthEast: (√3/2, −1/2)    NorthWest: (0,     −1)
SouthEast: (0,     1)      SouthWest: (−√3/2,  1/2)
```

Wind vectors from the wind field are expressed in the same frame so
dot products are geometrically correct.

### CFL stability criterion: |v| ≤ 0.25

For a tile with wind vector `v`, the maximum sum of positive dot
products across all six unit vectors is `2|v|` (achieved when wind
points directly toward two adjacent neighbours). The stability
condition for donor-cell advection is that total outflow `W ≤ 1`,
giving the strict bound `|v| ≤ 0.5`.

A conservative safety margin of half the strict bound is used:
`|v| ≤ 0.25`. This is validated at parameter load time by
`ClimateParams::validate`; a debug assertion in the wind field
constructor catches values that exceed 0.5 after noise addition.

### Precipitation: separate pass, not inline

Precipitation is computed as a separate sweep over the post-advection
moisture field after each advection step, not inline during the
advection loop. This separates the two phases cleanly:

```
loop:
    advection_step(moisture_in → moisture_out)   // distributes moisture
    precipitation_step(moisture_out in-place)    // removes fraction
    swap(moisture_in, moisture_out)
```

If precipitation were computed inline during advection, a tile visited
early in the iteration would see its upwind neighbours' moisture before
those neighbours have advected — and the precipitated amount would
depend on iteration order. The separate pass ensures every tile sees
the same post-advection moisture field regardless of when it was
visited.

### Orographic precipitation: one upwind neighbour

The orographic boost multiplies the precipitation fraction by
`1 + orographic_factor × max(0, Δelev)`, where `Δelev` is the
elevation gain from the tile that is most opposed to the wind direction
(the "upwind neighbour"). Only one neighbour is used, not an average
or a per-direction weighted sum.

This is the simplest semantics that produces the correct physical
effect: a tile higher than the air arriving from upwind forces that air
to rise, causing enhanced precipitation. Using a single upwind
neighbour avoids ambiguity when wind points between two neighbour
directions and is sufficient for the visible rain-shadow effect that is
the tuning target.

## Consequences

**Positive:**

- Donor-cell advection is positive-definite: the advection step alone
  never produces negative moisture values. Combined with the separate
  precipitation step (which only removes moisture), the moisture field
  remains non-negative throughout.
- The scheme is simple to implement and to reason about. Each tile's
  contribution to its neighbours is a straightforward projection.
- The CFL bound, validated at parameter load time, prevents numerical
  instability without any runtime checks in the hot loop.
- The separate precipitation pass eliminates iteration-order dependence
  on orographic enhancement, giving a clean result that is independent
  of the `(r, q)` traversal order used for determinism.

**Negative:**

- Donor-cell advection is first-order accurate and numerically
  diffusive. Moisture fronts smear over several tiles rather than
  remaining sharp. At the scales involved (240-wide maps with ~1.5
  transits), this diffusion is acceptable and partly desirable — it
  smooths out the pixelated appearance of a sharp advection front.
- The CFL cap (|v| ≤ 0.25) is conservative. The strict bound is
  |v| ≤ 0.5; the 0.25 cap wastes half the available wind speed range.
  This was chosen because wind noise can push individual tile magnitudes
  above the base `wind_amplitude`, and the debug assertion at 0.5 is
  the actual safety net.

## Alternatives considered

**Semi-Lagrangian advection** — trace each tile's characteristic
backward in time to find its source point and interpolate moisture from
the source point. Unconditionally stable (no CFL constraint), allows
larger wind amplitudes. Rejected because trilinear interpolation between
hex tiles is significantly more complex to implement correctly, and
interpolation introduces its own numerical diffusion that is harder to
characterise. Revisit if the CFL cap ever becomes a design constraint.

**Higher-order upwind (MUSCL, PPM)** — use slope-limited flux
reconstruction for sharper advection fronts. Rejected for the same
reason as semi-Lagrangian: added complexity without a clear benefit at
the map sizes and iteration counts involved.

**Inline precipitation** — compute the precipitation fraction for each
tile immediately after distributing its outflow during the advection
pass. Simpler code, one fewer field. Rejected because the precipitation
fraction uses the orographic boost, which reads the elevation of the
upwind neighbour — the same neighbour that may not have been visited
yet in the iteration order. This creates iteration-order dependence
that is subtle and hard to reason about.

**Per-deposit orographic accounting** — instead of looking up one
upwind neighbour, accumulate elevation gain across the full upwind path
that moisture has travelled. More physically accurate: moisture that
has already risen over a ridge should not get a second orographic boost
on the leeward side. Rejected because it requires tracking provenance
of moisture across iterations, adding significant bookkeeping without
a clear visual improvement at the tuning stage.

## References

- `crates/worldgen/src/stages/climate/advection.rs` — advection kernel
- `crates/worldgen/src/stages/climate/precipitation.rs` — precipitation kernel
- `crates/worldgen/src/stages/climate/wind.rs` — `HEX_UNIT_VECTORS` constants
- `crates/worldgen/src/params/climate.rs` — `CFL_MAX_WIND_AMPLITUDE` constant
- ADR-0006 — fixed-count iteration (context for the loop structure)
- STATUS.md §3.2 — mathematical model overview
