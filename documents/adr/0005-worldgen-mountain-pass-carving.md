# ADR-0005: Mountain Pass Carving (Step 10)

**Status:** Accepted  
**Date:** 2026-05-25  
**Deciders:** Kristoffer

## Context

Convergent plate boundaries produce continuous linear mountain ridges.
The elevation pipeline (step 6–7) stamps a Gaussian elevation modifier at
every boundary tile; for a convergent boundary those modifiers sum to a
ridge whose peak elevation is proportional to `mountain_elevation_boost`
and whose width scales with `boundary_falloff_sigma`. Reducing sigma
narrows the ridge but cannot break it into disconnected peaks: the
boundary is a connected chain of tiles, each stamping its own Gaussian,
so the integrated ridge is continuous as long as the boundary is.

The consequence for gameplay is that mountain ranges form impassable
walls. Units cannot cross them; civilizations on opposite sides of a
convergent boundary are effectively isolated for the early game. Some
isolation is desirable — mountain ranges should be meaningful barriers —
but total impassability across entire continents is gameplay-breaking.

Real mountain ranges have passes: lower-elevation gaps where rivers,
glaciers, and trade routes historically crossed. These arise from
geological heterogeneity that the simplified pipeline does not model.
A targeted post-processing step can simulate their effect.

The original proposal was to address this structurally by reducing
`boundary_falloff_sigma` further. That was tried (sigma reduced from 8.0
to 2.0–3.0). It produced narrower ranges but not fewer walls: at any
sigma, a connected boundary line produces a connected ridge at mountain
level. The quantile thresholding (step 8) then places the mountain/high-
land threshold at the right elevation to hit `level_proportions[last]`
regardless of ridge shape. Structural fixes cannot solve a topological
problem; a post-processing step is necessary.

## Decision

Add step 10 to the tectonics pipeline: **mountain pass carving**.

After height-level quantization (step 8) and before the hills pass
(step 9 becomes step 10 in the renamed sequence — see note below), iterate
over every mountain-level tile and evaluate a wrap-aware `OpenSimplex`
noise sample. Tiles whose sample exceeds `mountain_pass_threshold` are
candidates for demotion from mountain to high-land.

### Neighbor-aware filter

When `mountain_pass_neighbor_aware` is `true` (the default), a tile is
only eligible for demotion if at least `mountain_pass_min_mountain_neighbors`
of its six hex-neighbors are also at mountain level. This restricts
demotion to the range interior: tiles already on the edge of the mountain
band (adjacent to high-land) are left alone. The result is passes that
punch through ranges rather than nibbling their perimeters.

When `mountain_pass_neighbor_aware` is `false`, demotion applies to any
mountain tile, including edge tiles. This produces a rougher, more eroded
appearance — appropriate for heavily weathered ranges — at the cost of
thinner ranges overall.

### Pipeline position

The pass runs after step 8 (quantization) because it operates on the
discrete `height_level` field. Running it before quantization would
require choosing a continuous elevation threshold, which is less
intuitive to tune and would interact with the quantile logic in non-
obvious ways. Running it after quantization means the pass is
transparent: `height_level` is modified in place with no new field added
to `TectonicData`, and the existing `31_08_height_levels_*.png` debug
outputs automatically reflect the result.

Hills classification (previously step 9, now step 10 in implementation
order) runs after the pass. The hills pass reads `boundary_type`, not
`height_level`, so the mountain pass does not affect hills classification.

### Step numbering

The module-level documentation renumbers the pipeline: what was "step 10
(Hills)" in the original comment becomes "step 11 (Mountain pass
carving)" at step 10, "step 11 (Hills)" — no, the hills remain step 10
and mountain passes are step 11 in the pipeline description, while the
implementation list in `tectonics.rs` now reads 1–11. The PNG filename
prefixes (`31_08_height_levels_px.png`, `31_09_hills_px.png`) are
unchanged; no new PNG is introduced.

### Effect on `level_proportions`

This pass demotes mountain tiles after quantization, so the achieved
mountain fraction will be below the `level_proportions[last]` target by
the fraction of mountain tiles demoted. To compensate, set
`level_proportions[last]` modestly higher than the desired final mountain
fraction. At the default threshold of `0.15`, approximately 35–40 % of
qualifying interior tiles are demoted. If `level_proportions[last] =
0.07` (previously `0.05`), the final mountain fraction after passing will
land near `0.04–0.05`.

This accepted imprecision is preferable to the alternative (running
quantization after the pass), which would require a second quantile
computation and would break the clean separation between the elevation
field and the discrete level field.

### Disabling the pass

Setting `mountain_pass_threshold > 1.0` disables the pass entirely. No
`OpenSimplex` sample exceeds `1.0`, so no tile is demoted. The preset
`default.ron` uses `0.15`; users who want impassable mountain walls can
set this to `1.1` in their preset.

### New parameters

Four new fields are added to `TectonicsParams`:

| Field | Type | Default | Range / note |
|---|---|---|---|
| `mountain_pass_threshold` | `f32` | `0.15` | Must be finite; `> 1.0` disables |
| `mountain_pass_noise_frequency` | `f32` | `3.0` | Must be finite and `> 0.0` |
| `mountain_pass_neighbor_aware` | `bool` | `true` | Not sweepable as scalar |
| `mountain_pass_min_mountain_neighbors` | `u8` | `3` | `0`–`6` |

`mountain_pass_min_mountain_neighbors` is sweepable as a scalar (rounded
before cast). `mountain_pass_neighbor_aware` must be set via `--params`.

## Consequences

**Positive:**

- Mountain ranges acquire traversable passes without any change to the
  elevation model or the Gaussian falloff logic.
- The pass is tunable and skippable: setting the threshold above `1.0`
  recovers the prior behavior exactly.
- Implementation is ~80 lines following the `hills.rs` pattern. No new
  data structures, no new fields on `TectonicData`, no new PNG outputs.
- The neighbor-aware filter localizes demotion to range interiors,
  preserving the visual mass of the range while opening interior routes.

**Negative:**

- The achieved mountain fraction diverges from `level_proportions[last]`
  by the demotion rate. The compensating instruction (raise the target
  proportion slightly) adds a non-obvious interaction between two
  parameters. This is documented in the parameter rustdoc.
- Adding a step between quantization and hills slightly complicates the
  mental model of "step 8 produces `height_level`; all subsequent steps
  read it." The mountain pass modifies `height_level` after quantization,
  so step 8 is no longer the single writer. This is documented in the
  `world_data.rs` step roster comment.
- `mountain_pass_neighbor_aware` is a bool and cannot be swept with the
  `maptest sweep` subcommand. Two separate preset files are needed to
  compare the two modes.

## Alternatives considered

**Structural: reduce `boundary_falloff_sigma` further** — tried; does
not break the connectivity of the ridge. A Gaussian at sigma = 1 is
still centered on each boundary tile; the ridge remains connected as long
as the boundary is. Rejected as structurally unable to solve the problem.

**Pre-quantization elevation dampening** — apply a noise-modulated
multiplier to high elevations before quantization, attenuating some
mountain-candidate tiles below the threshold. Cleaner mathematically
(preserves the `level_proportions` invariant) but the demotion criterion
("this tile is interior to a range") is only computable after
quantization. Without the neighbor filter, pre-quantization dampening
would thin ranges from both edges and interior uniformly, eroding the
perceived mass of mountain ranges. Rejected; the post-quantization
approach allows the neighbor filter.

**River-carved passes** — defer pass generation to the hydrology stage
(Stage 3.3), where rivers that cross mountain tiles could explicitly
lower those tiles. Geologically accurate, but requires the hydrology stage
to exist before maps are playable. Rejected for now; revisit in Stage 3.3
to see whether hydrological passes make the mountain pass carving step
redundant.

## Revisit condition

If Stage 3.3 (hydrology) implements river-carved passes that adequately
break up mountain ranges, this step may become redundant. At that point,
evaluate whether to retain it as a supplementary roughening pass or
supersede this ADR with one that removes the step.

## References

- Design doc §1.9 — tectonics pipeline, step 8 (quantization)
- `crates/worldgen/src/stages/tectonics/mountain_pass.rs` — implementation
- `crates/worldgen/src/params/tectonics.rs` — new parameters
- ADR-0004 — multi-octave boundary perturbation (companion change)
- Stage 3.3 design notes — hydrology, river formation
