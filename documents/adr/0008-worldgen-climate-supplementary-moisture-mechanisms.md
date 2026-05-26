# ADR-0008: Supplementary Moisture Transport Mechanisms

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

The planned climate model was a three-phase loop: evaporation sources
inject moisture into water tiles, advection carries it downwind, and
precipitation removes a fraction per tile per iteration. This is a
physically coherent model, but tuning revealed four failure modes that
the three-phase loop cannot address structurally.

**Failure mode 1: Coastal land tiles stay dry when wind runs parallel to
the coast.** Advection distributes moisture along the wind direction.
A land tile adjacent to ocean that lies parallel to the wind receives
little or no upwind moisture; the ocean evaporates but the moisture
bypasses the coast entirely. This produces implausible desert coastlines
in regions where the wind blows along the shore.

**Failure mode 2: Moisture decays too fast above land.** Each land tile
precipitates a fraction of its moisture per iteration. At any viable
`base_precipitation_rate`, moisture decays geometrically hop-by-hop and
is exhausted within 10–20 tiles inland. The result is a wet coastal
strip and a dry interior regardless of map size or iteration count.
Real atmospheres partially counteract this through plant transpiration
and soil evaporation: moisture that precipitates on land re-enters the
atmosphere and continues inland.

**Failure mode 3: Zonal wind cannot form N-S latitude bands.** The base
wind profile `u(φ) = −A·cos(3π·φ)` is purely zonal (east-west). N-S
wind comes only from the noise perturbation. At viable noise amplitudes
(constrained by the CFL cap) the N-S wind component is weak. The
moisture field therefore develops strong E-W bands determined by
advection distance from ocean sources, but not the N-S latitude bands
(wet tropics, dry subtropics, wet mid-latitudes) that the design
requires.

**Failure mode 4: Ocean tiles dominate the moisture field.** Over many
iterations, ocean and coastal land tiles accumulate moisture that is
never fully drained by precipitation (there is no precipitation on open
ocean in the model). Their values grow large, compressing interior land
values toward zero. Visualizations show a near-uniform maximum along
coasts and a near-zero interior — correct in relative terms but
useless for calibration.

## Decision

Four supplementary mechanisms are added to `ClimateParams`, all
defaulting to 0.0 or disabled so that the baseline three-phase model
is unchanged at default settings.

### Coastal moisture source (`coastal_moisture_factor`, step 3b)

Each land tile receives `coastal_moisture_factor × N` additional
moisture per iteration, where `N` is the count of its hex neighbours
that are water tiles (ocean or coast). This source is added directly
to `moisture_out` in the advection step, alongside the evaporation
source.

This addresses failure mode 1. The mechanism is wind-independent:
coastal land tiles receive moisture regardless of wind direction,
modelling sea breeze and coastal fog. The per-neighbour scaling means
peninsulas and narrow coastal strips receive proportionally more
moisture than tiles with only one water neighbour.

### Evapotranspiration (`evapotranspiration_rate`, step 4b)

Each land tile re-emits `evapotranspiration_rate × M` of its moisture
back into the atmosphere per iteration, where `M` is the tile's
moisture at the start of the iteration. This re-emitted moisture is
added to `moisture_out` for the same tile (it does not leave the tile;
it re-enters the atmosphere and will be advected and precipitated in
future iterations).

This addresses failure mode 2. Rather than moisture decaying to zero
after the first precipitation, a fraction is recycled and available
to advect further inland. The effect is that inland tiles receive a
sustained trickle of moisture from the wet coastal strip, rather than
receiving nothing once coastal moisture has precipitated.

### Moisture cap (`moisture_cap`, step 3c)

After each advection step, every tile's moisture is clamped to
`moisture_cap`. Tiles that would exceed the cap lose the excess
(moisture is removed from the system).

This addresses failure mode 4. A finite cap prevents ocean and coastal
tiles from dominating the field. Setting `moisture_cap` to a value
comparable to a few iterations' worth of evaporation (e.g. `50.0` for
`evaporation_rate = 1.0`) keeps coastal values bounded and makes
interior values visible in the same colormap range. `f32::MAX` is
used as the sentinel for "disabled" to avoid adding a separate boolean
field; the equality check against `f32::MAX` is explicit and uses an
`#[allow(clippy::float_cmp)]` annotation.

### Isotropic diffusion (`diffusion_rate`, step 4c)

Each tile distributes `diffusion_rate / 6` of its moisture equally to
each of its six hex neighbours, in addition to the wind-driven outflow.
This is computed in the same advection pass: the total outflow fraction
from each tile is `W + D` where `W` is the wind outflow fraction and
`D` is `diffusion_rate`.

This addresses failure mode 3. Isotropic diffusion is direction-
independent and transports moisture N-S as effectively as E-W. The
effect is a blurring of the moisture field that spreads moisture from
wet regions (equatorial ocean, coastal strips) into adjacent dry
regions (subtropical bands, interiors), allowing latitude band
structure to emerge even with a weak N-S wind.

**Interaction with the CFL bound.** Adding diffusion changes the
stability condition. Without diffusion, the total outflow fraction `W`
must be ≤ 1 for stability, giving `|v| ≤ 0.5` (strict CFL) and
`|v| ≤ 0.25` (used conservative cap). With diffusion, the total
outflow is `W + D`, so the combined stability condition is `W + D ≤ 1`.
Since `W ≤ 0.5` (from the wind CFL cap) and `D ≤ 0.5` (the validation
cap on `diffusion_rate`), the worst-case sum is exactly 1.0. The
advection kernel guards against floating-point overshoot with a
`.max(0.0)` on the retained fraction.

The `diffusion_rate` validation range `[0.0, 0.5]` is derived from
this constraint, not from a physical argument.

## Consequences

**Positive:**

- All four mechanisms default to 0.0 or disabled. The baseline three-
  phase model is byte-identical at defaults. Reference hashes are not
  broken by adding the parameters.
- Each mechanism targets a distinct failure mode and can be tuned
  independently. Sweeping one does not require compensating changes to
  the others (they interact additively, not multiplicatively).
- Evapotranspiration and coastal source together allow viable moisture
  distribution at `base_precipitation_rate` values that are physically
  plausible (0.01–0.05) rather than requiring an artificially low rate
  to prevent inland decay.
- Isotropic diffusion enables N-S latitude band formation without
  requiring large N-S wind components that would approach the CFL cap.

**Negative:**

- Four additional parameters increase the tuning surface. Each preset
  now requires considered values for `coastal_moisture_factor`,
  `evapotranspiration_rate`, `moisture_cap`, and `diffusion_rate`.
  The default of 0.0 is a starting point, not a recommended value.
- The moisture cap removes moisture from the system, breaking the
  conservation property of the pure advection model. Accumulated
  precipitation totals no longer equal total evaporation over the
  run. This is acceptable — the model is a steady-state approximation,
  not a water-budget simulation — but it means field values should not
  be interpreted as physically meaningful quantities.
- Isotropic diffusion smooths the moisture field. In regions where
  sharp transitions are desirable (e.g., the boundary of a rain shadow
  behind a mountain range), diffusion softens the edge. Tuning
  `orographic_factor` higher can compensate, but the interaction
  requires care.
- The `f32::MAX` sentinel for `moisture_cap` requires an explicit
  `#[allow(clippy::float_cmp)]` at the equality check. This is a small
  code smell; a separate `moisture_cap_enabled: bool` field would be
  cleaner but was rejected as over-engineering for a single flag.

## Alternatives considered

**Stronger N-S noise on the wind field** — increase `wind_noise_amplitude`
to force N-S wind components. Rejected because `wind_noise_amplitude`
is constrained by the CFL cap: at viable noise amplitudes the N-S
component remains small relative to the zonal base. Raising the cap
would require reducing `wind_amplitude`, weakening the three-band
profile that is the design target.

**Precipitation on ocean tiles** — allow ocean tiles to precipitate,
draining their moisture and preventing accumulation. Rejected because
precipitation on ocean has no gameplay significance and makes the
accumulated precipitation field meaningless over the ocean. The moisture
cap achieves the same visual result without semantic cost.

**Explicit moisture recycling route** — rather than re-emitting
evapotranspiration moisture to the same tile, inject it into a
dedicated secondary evaporation field that advects separately from the
land tile. Rejected as over-engineering: the single re-emission to the
same tile is sufficient for the tuning target (moisture reaching 50+
tiles inland) and avoids a second field allocation.

## References

- `crates/worldgen/src/stages/climate/advection.rs` — coastal source,
  evapotranspiration, moisture cap, and diffusion in the advection kernel
- `crates/worldgen/src/stages/climate/evaporation.rs` — `build_coastal_source_field`
- `crates/worldgen/src/params/climate.rs` — new parameter fields and validation
- `tools/maptest/presets/default.ron` — tuned values for all four mechanisms
- ADR-0007 — donor-cell advection (CFL context for the diffusion constraint)
