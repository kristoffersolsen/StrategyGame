# ADR-0015: Boundary Effect Noise for Tectonic Elevation

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer  
**Amends:** ADR-0005 (mountain pass carving remains available, but
range breaking should primarily happen before height quantization)

## Context

The tectonic elevation pipeline applies boundary effects by visiting
each tectonic boundary source tile, computing an elevation modifier from
the local plate types and boundary type, and spreading that modifier
outward with a Gaussian falloff controlled by
`boundary_falloff_sigma`.

This produces readable tectonic structure, but it also creates a visible
artifact: mountain ranges, island arcs, rifts, and trenches can appear as
continuous uniform bands. Every boundary source contributes with the
same strength and the same falloff width, so connected boundaries tend
to become connected terrain features.

ADR-0005 addressed the gameplay problem for mountains by adding
post-quantization mountain pass carving. That opened routes through
continuous mountain walls, but it operates after height levels have
already been assigned. As a result, it changes the discrete
`height_level` field after the quantile pass and cannot shape the
continuous elevation field itself.

The desired behavior is to keep terrain features aligned to tectonic
boundaries while making them less uniform:

- variation along boundary chains should split ranges into peaks, gaps,
  and island chains;
- variation in falloff width should make some sections narrow and sharp
  while others are broad and diffuse;
- the implementation should avoid explicit local boundary geometry unless
  visual tuning proves it is needed.

## Decisions

### 1. Add source-strength noise to boundary effects

`TectonicsParams` gains:

```text
boundary_strength_noise_amplitude: f32
boundary_strength_noise_frequency: f32
```

For each boundary source tile, the elevation pipeline samples coherent
wrap-aware `OpenSimplex` noise at the source coordinate. The noise value
is remapped into a smooth mask and used to scale the source modifier
before it is propagated:

```text
contribution = modifier * strength_scale * gaussian(distance, local_sigma)
```

This is not a true tangent-coordinate projection. Instead, the noise is
sampled only at boundary source tiles. Because only boundary tiles emit
the modifier, spatial noise sampled at those sources behaves visually as
along-boundary variation while keeping the implementation small and
cheap.

`boundary_strength_noise_amplitude = 0.0` is an exact no-op and returns
`strength_scale = 1.0`.

Validation requires:

```text
0.0 <= boundary_strength_noise_amplitude <= 1.0
boundary_strength_noise_frequency > 0.0
```

### 2. Add per-source sigma noise to boundary effects

`TectonicsParams` gains:

```text
boundary_sigma_noise_amplitude: f32
boundary_sigma_noise_frequency: f32
```

For each boundary source tile, the elevation pipeline samples a second
coherent wrap-aware `OpenSimplex` noise field and uses it to scale that
source tile's local Gaussian falloff sigma:

```text
local_sigma = boundary_falloff_sigma * sigma_scale
```

This approximates orthogonal variation without computing explicit
boundary normals. A narrow local sigma creates sharper, tighter sections;
a wider local sigma creates broader, softer sections.

The local sigma scale is clamped internally to keep the propagation
radius bounded:

```text
0.35 <= sigma_scale <= 1.75
```

`boundary_sigma_noise_amplitude = 0.0` is an exact no-op and returns
`local_sigma = boundary_falloff_sigma`.

Validation requires:

```text
0.0 <= boundary_sigma_noise_amplitude <= 1.0
boundary_sigma_noise_frequency > 0.0
```

### 3. Keep defaults output-preserving

The new struct defaults are:

```text
boundary_strength_noise_amplitude = 0.0
boundary_strength_noise_frequency = 3.0
boundary_sigma_noise_amplitude = 0.0
boundary_sigma_noise_frequency = 3.0
```

With both amplitudes at zero, boundary-effect noise is not sampled and
the boundary contribution is byte-identical to the prior implementation.

The boundary noise seed is derived from the existing elevation seeds
rather than drawn directly from the stage RNG. This avoids changing the
stage RNG stream for default worlds and preserves the committed
determinism reference hash.

### 4. Keep mountain-pass carving for now

ADR-0005 remains accepted. `mountain_pass` is still part of the pipeline
and remains useful while the new boundary-noise parameters are tuned.

However, boundary source-strength noise is now the preferred first lever
for breaking continuous mountain chains because it acts before
quantization. Mountain-pass carving should become either a secondary
gameplay pass or a candidate for deprecation after tuning confirms that
boundary noise creates enough natural gaps.

### 5. Expose the new controls through tuning tools

The new parameters are included in:

- `tools/maptest/presets/default.ron`;
- `tools/maptest/presets/test.ron`;
- `tools/maptest/presets/more_land_levels.ron`;
- `tools/maptest/src/sweep.rs`;
- `scripts/mapgen_sweep.sh` as commented sweep candidates;
- `documents/crates/worldgen/parameters/tectonics.md`.

Recommended initial sweeps:

```text
boundary_strength_noise_amplitude: 0.0..1.0
boundary_strength_noise_frequency: 1.0..8.0
boundary_sigma_noise_amplitude: 0.0..1.0
boundary_sigma_noise_frequency: 1.0..8.0
```

## Consequences

**Positive:**

- Continuous boundary-aligned terrain features can break into chains and
  gaps before quantization.
- Mountain ranges, island arcs, rifts, and trenches can vary in width
  without explicit tangent/normal geometry.
- Defaults preserve existing output and the committed determinism
  reference hash.
- The implementation stays local to `add_boundary_effects`, matching the
  place where the uniform-band artifact originates.
- The approach is cheaper and simpler than computing boundary orientation
  metadata for every boundary tile.

**Negative:**

- The source-strength noise is not mathematically tangent-aligned. It is
  spatial noise sampled on boundary source tiles, so it may need tuning
  if it reads as blotchy rather than chain-like.
- Sigma variation changes the effective propagation radius. The code
  clamps the scale, but high amplitudes still increase work by expanding
  the maximum search radius.
- The mountain-pass system and boundary-strength noise now overlap in
  purpose. Keeping both is useful for tuning, but the long-term ownership
  of "mountain gaps" should be revisited.
- The noise is applied to any non-zero boundary modifier, including
  negative trench and rift modifiers. This is intentional for now, but
  uplift-only and depression-only controls may be useful later.

## Alternatives considered

**True tangent/normal projection** - derive a local tangent and normal
for each boundary tile, then sample separate noise fields in those
coordinate frames. Rejected for the first implementation because
`add_boundary_effects` does not currently retain boundary orientation
metadata. The simpler source-tile model produces the intended visual
effect with less code, less state, and lower per-tile cost.

**Keep relying on mountain-pass carving** - continue to demote mountain
tiles after quantization. Rejected as the primary solution because it
addresses only discrete mountain connectivity. It cannot vary the
continuous elevation field, island arcs, trenches, or rifts before
height levels are assigned.

**Add noise to the final height-level field** - perturb discrete
height levels near boundaries after quantization. Rejected because it
would bypass the elevation field and introduce another post-processing
step that can fight the quantile proportions.

**Apply one global detail-noise field only** - raise
`detail_noise_amplitude` and rely on general terrain noise to break
ranges. Rejected because global detail noise affects interiors and
coastlines as much as boundary terrain. The artifact is boundary-local,
so the correction should be boundary-local.

## Revisit conditions

- If strength noise creates blotchy gaps instead of coherent range
  segmentation, add explicit boundary orientation metadata and switch to
  tangent-coordinate sampling.
- If sigma noise noticeably increases generation cost on large maps,
  reduce the max sigma clamp or precompute boundary source records.
- If tuned boundary noise reliably replaces post-quantization pass
  carving, supersede ADR-0005 and either disable `mountain_pass` by
  default or remove it.
- If trenches and rifts need different behavior from uplift features,
  split the controls into uplift and depression variants.

## References

- ADR-0005 - mountain pass carving.
- ADR-0014 - plate growth variance and plate-type clumping.
- `crates/worldgen/src/stages/tectonics/elevation.rs` -
  `add_boundary_effects`, source-strength noise, and local sigma noise.
- `crates/worldgen/src/params/tectonics.rs` - new parameters and
  validation.
- `tools/maptest/src/sweep.rs` - sweep exposure.
- `documents/crates/worldgen/parameters/tectonics.md` - tuning documentation.
