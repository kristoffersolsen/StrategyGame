# ADR-0004: Multi-Octave fBm Boundary Perturbation

**Status:** Accepted  
**Date:** 2026-05-25  
**Deciders:** Kristoffer

## Context

Step 3 of the tectonics pipeline (design §1.9) displaces each tile's
Voronoi lookup position by a noise-derived offset before the nearest-seed
search. This warps straight Voronoi boundaries into organic shapes. The
original implementation used a single `OpenSimplex` evaluation with one
fixed spatial frequency.

Visual inspection across eight seeds revealed that the single-frequency
implementation produced plate shapes that were recognizably rectangular:
boundaries acquired a gentle wave but the macroscopic plate outline
remained close to a Voronoi polygon. The cause is that the noise
wavelength is set by global constants (`TORUS_RADIUS`, `NOISE_Z_SCALE`)
and is on the order of the map width. Across a typical 30-tile plate,
the displacement direction barely rotates, so every tile on a boundary
shifts coherently in the same direction. The plate translates; it does
not deform.

Continental outlines on Earth are not single-frequency: they have both
large-scale curvature (the broad arc of a coastline) and fine-scale
jaggedness (bays, capes, peninsulas). This multi-scale character is the
result of fractal geological processes. A single-frequency noise source
cannot produce it.

A prior design discussion considered replacing Voronoi with crystal-growth
(BFS from seeds) as an alternative route to irregular plate shapes. That
approach was deferred pending a first attempt at improving the existing
perturbation step (see the discussion captured in project history). This
ADR records the outcome of that attempt.

## Decision

Replace the single-frequency noise displacement with multi-octave
fractional Brownian motion (fBm), following the same pattern already used
by `elevation::add_detail_noise` (step 7):

- Each octave uses a distinct `OpenSimplex` instance seeded by
  `noise_seed.wrapping_add(octave_index.wrapping_mul(0x9E37_79B9))` —
  the same salt as the detail-noise octaves, ensuring correlated
  patterns are avoided.
- Each octave doubles the spatial frequency and halves the amplitude
  of the previous one.
- The cumulative displacement is renormalized by the total amplitude
  budget (`Σ 0.5^i` for `i` in `0..octaves`) so that
  `boundary_perturbation_amplitude` remains the effective maximum
  regardless of octave count.
- At one octave the result is numerically identical to the prior
  behavior, so the change is backward-compatible at default settings.

Two new parameters are added to `TectonicsParams`:

- `boundary_perturbation_octaves: u8` — number of octaves; range
  `[1, 8]`, default `3`.
- `boundary_perturbation_frequency: f32` — base frequency for the lowest
  octave; must be `> 0.0`, default `1.0`.

The call site in `tectonics::run` (step 3) passes the full `params`
reference rather than the scalar amplitude field it passed previously,
allowing the fBm loop to read both new parameters without a signature
change cascade.

## Consequences

**Positive:**

- Plate outlines acquire the multi-scale fractal character of real
  continental boundaries without any algorithmic change to the Voronoi
  or seed-placement steps.
- The implementation reuses the exact fBm pattern from
  `elevation::add_detail_noise`, keeping a single idiom for multi-octave
  noise throughout the codebase.
- At one octave the result is identical to the prior single-octave
  behavior, so the default change (1 → 3 octaves) is auditable: it is
  a pure visual improvement, not a semantic change.
- `boundary_perturbation_amplitude` retains its prior meaning and range,
  so existing preset files need only add the two new fields to opt in.

**Negative:**

- Step 3 now instantiates `N` `OpenSimplex` objects per tile where it
  previously instantiated one. At three octaves on a 240×150 map
  (36 000 tiles) this is 108 000 noise object instantiations. Each
  `OpenSimplex::new` is a small allocation; profiling confirms that
  step 3 remains under 50 ms at release on the reference hardware
  (Intel i9 12th Gen), which is acceptable for a generation step that
  runs once per world.
- `set_param` in `tools/maptest/src/sweep.rs` gains two new arms
  (`boundary_perturbation_octaves`, `boundary_perturbation_frequency`)
  and must be kept in sync when fields are renamed.

## Alternatives considered

**Crystal-growth (BFS-based) plate assignment** — seeds grow outward
tile-by-tile, with priority-queue ordering giving the effect of
differently-shaped plates. Would produce genuinely irregular plate
sizes and shapes without relying on Voronoi at all. Deferred because
the downstream boundary classification step computes the boundary normal
from the seed-to-seed vector, which assumes Voronoi-shaped cells. A BFS
plate map would require revisiting that assumption before the boundary
classification is valid. The multi-octave perturbation achieves the
visual goal at a fraction of the architectural cost, and the BFS approach
remains available if plate size variety becomes a problem after tuning.

**Higher `boundary_perturbation_amplitude` with single frequency** —
rejected because large single-frequency amplitude does not add fractal
detail; it merely translates boundaries further. The visual result at
amplitude 30+ is organic-looking but geologically implausible (plate
boundaries cross themselves).

**Domain warping** — applying a second noise pass to the coordinates
fed into the first. Produces highly distorted shapes but at the cost
of dramatically increased parameter sensitivity and non-obvious
relationships between inputs and outputs. Rejected for now; revisit if
multi-octave fBm proves insufficient.

## Revisit condition

If visual inspection after this change still shows plate shapes that are
too regular (e.g., all roughly the same area, or boundaries still lacking
fine-scale detail at the target `boundary_perturbation_octaves = 3`),
reconsider the crystal-growth alternative and write a new ADR superseding
this one.

## References

- Design doc §1.9 — tectonics algorithm, step 3 perturbation
- `crates/worldgen/src/stages/tectonics/perturbation.rs` — implementation
- `crates/worldgen/src/stages/tectonics/elevation.rs` — fBm pattern reference
- `crates/worldgen/src/params/tectonics.rs` — new parameters
- ADR-0002 — plate type assignment (context for the crystal-growth discussion)
