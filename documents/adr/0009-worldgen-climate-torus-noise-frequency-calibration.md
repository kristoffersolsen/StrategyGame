# ADR-0009: Torus-Lifting Geometry and Wind Noise Frequency Calibration

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

The wind field adds a low-frequency OpenSimplex noise perturbation to
the base zonal profile to break up the perfectly uniform E-W bands.
Because the map wraps east-west, naive 2D noise would produce a
discontinuity at the wrap seam. The solution — already established in
the tectonics elevation module — is torus lifting: the q-coordinate is
mapped to a point on a circle before noise evaluation, so the noise
field is seamless at the wrap boundary.

The mapping is:

```
angle  = 2π × q / map_width
nx     = TORUS_RADIUS × cos(angle) × wind_noise_frequency
ny     = TORUS_RADIUS × sin(angle) × wind_noise_frequency
nz     = r × NOISE_Z_SCALE       × wind_noise_frequency
```

where `TORUS_RADIUS = 1/(2π) ≈ 0.159`.

The plan described `wind_noise_frequency` as controlling "spatial
frequency in fractions of map width", expecting that `0.5` would
produce "continent-scale deflections". During tuning, the perturbation
at `wind_noise_frequency = 0.5` was negligible — visually
indistinguishable from zero noise — despite a nonzero `wind_noise_amplitude`.

## Decision

The root cause is geometric. The effective radius of the torus circle
is `TORUS_RADIUS × wind_noise_frequency`. Adjacent q-tiles are
separated by `1/map_width` of a full rotation on the circle, so their
noise inputs differ by:

```
Δnoise_input ≈ 2π × (1/map_width) × TORUS_RADIUS × wind_noise_frequency
             = wind_noise_frequency / map_width
```

At `wind_noise_frequency = 0.5` and `map_width = 240`:

```
Δnoise_input ≈ 0.5 / 240 ≈ 0.002
```

OpenSimplex noise varies on a scale of roughly 1.0 in its input space.
A tile-to-tile input difference of 0.002 produces a tile-to-tile output
difference of approximately `0.002 × gradient_magnitude` — effectively
zero at the noise amplitude scale. The torus circle is so small that
adjacent tiles lie almost at the same point on it.

At `wind_noise_frequency = 4.0`:

```
Δnoise_input ≈ 4.0 / 240 ≈ 0.017
```

The effective circle radius is `1/(2π) × 4.0 ≈ 0.64`. At this radius,
adjacent tiles are meaningfully separated in noise space and the
perturbation is visible at the expected amplitude.

**The practical rule:** `wind_noise_frequency` values below approximately
`map_width / 100` produce negligible perturbation due to the torus
radius scaling. For a 240-wide map, the threshold is roughly 2.4; the
tuned default of 4.0 comfortably exceeds it.

The `ClimateParams::defaults()` struct retains `wind_noise_frequency =
0.5` as its code default for backward compatibility with reference
hashes. The `tools/maptest/presets/default.ron` preset overrides this
to `4.0` for all actual generation runs, with the comment explaining
the geometry:

```ron
wind_noise_frequency: 4.0,  // raised from 0.5: torus circle radius = 1/(2π) × freq,
                              // so 0.5 gives radius≈0.08 (negligible N-S wind), 4.0 gives ≈0.64
```

The `TORUS_RADIUS` constant in `wind.rs` is documented with this
relationship so that future maintainers can recalibrate if the constant
changes.

## Consequences

**Positive:**

- Torus lifting is preserved: the noise field remains seamless at the
  east-west wrap boundary regardless of `wind_noise_frequency`. The
  fix is purely a calibration change, not an algorithmic one.
- The comment in `default.ron` makes the calibration discoverable
  without needing to trace through the math. Future maintainers tuning
  `wind_noise_frequency` will see the geometric explanation inline.
- The disconnect between `ClimateParams::defaults()` (0.5) and
  `default.ron` (4.0) is explicit and documented; it does not hide a
  bug, it records a deliberate calibration choice.

**Negative:**

- `ClimateParams::defaults()` returns a value (0.5) that is known to
  produce negligible noise effect. Any code that uses `defaults()` and
  does not load a preset file will silently get a near-zero perturbation.
  This is a latent footgun for future consumers of the `worldgen` crate.
- The disconnect between the struct default and the preset default
  means the "default" behavior depends on whether you use the code
  API directly or go through `maptest`/`mapdump`. This could be
  resolved by changing `ClimateParams::defaults()` to 4.0 and updating
  the reference hash — deferred until reference hashes are activated at
  Stage 3.2 completion.

## Revisit condition

When the Stage 3.2 determinism test is activated and a reference hash
is committed, update `ClimateParams::defaults()` to `wind_noise_frequency:
4.0` in the same commit that locks the hash. At that point the
disconnect between struct default and preset default is resolved, and
the backward-compatibility rationale for keeping 0.5 disappears.

## Alternatives considered

**Change `TORUS_RADIUS`** — instead of calibrating `wind_noise_frequency`,
adjust `TORUS_RADIUS` to a value that makes `frequency = 1.0` correspond
to a useful spatial scale. Rejected because `TORUS_RADIUS = 1/(2π)`
gives the torus circumference exactly 1.0, matching the step along `r`
in `NOISE_Z_SCALE`. Changing it breaks the geometric interpretation of
the constant and would misalign the tectonics elevation module (which
uses the same constant) without gaining anything.

**Separate N-S and E-W noise frequencies** — expose `wind_noise_frequency_q`
(east-west) and `wind_noise_frequency_r` (north-south) as separate
parameters, with different torus-lifting geometry for each. Rejected as
over-parameterisation; the single-frequency approach is sufficient and
the calibration problem is solved by using 4.0.

**Replace torus lifting with a projected 2D approach** — use the map's
x coordinate directly without torus lifting, and simply accept the seam.
Rejected because the seam artefact is visible in texture maps as a
sharp line at the wrap boundary.

## References

- `crates/worldgen/src/stages/climate/wind.rs` — torus-lifting implementation,
  `TORUS_RADIUS` and `NOISE_Z_SCALE` constants
- `tools/maptest/presets/default.ron` — `wind_noise_frequency: 4.0` with
  geometry comment
- `crates/worldgen/src/stages/tectonics/elevation.rs` — original torus-lifting
  pattern that `wind.rs` mirrors
- ADR-0008 — supplementary mechanisms (context: diffusion was added partly
  because wind noise alone could not produce N-S moisture transport)
