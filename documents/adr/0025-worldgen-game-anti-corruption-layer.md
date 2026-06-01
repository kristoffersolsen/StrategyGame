# ADR-0025: Anti-corruption layer between worldgen and game

**Status:** Accepted
**Date:** 2026-05-30
**Deciders:** Kristoffer

## Context

`worldgen` produces `WorldData`, the output of the generation pipeline.
Until now `game` has consumed `WorldData` directly as its runtime
representation of the map, reading the fields it needs and ignoring the
rest.

This couples `game` to `worldgen`'s internal data format. `WorldData`
carries generation-internal fields that `game` has no legitimate interest
in (moisture, tectonic and climate intermediates, generation scratch).
More importantly, every field `game` reads is an implicit contract:
`worldgen` cannot evolve its internal representation for generation reasons
without risking a recompile or breakage in `game`, even for fields `game`
should never have seen.

The motivation for separating the two is **not** memory. The unused fields
amount to a few megabytes over the default map — negligible. If memory were
the only concern, the right answer would be to keep `WorldData` as a
long-lived resource and ignore the unused fields. The concern is
**coupling**: the two crates' data models cannot evolve independently while
`game` depends on `worldgen`'s shape.

A naive fix — define `game`'s model as "`WorldData` with unused columns
removed" — does not actually decouple, because the result is still
`worldgen`'s shape, narrower, and drifts back toward coupling as soon as
convenience tempts reaching for one more field.

Two facts make `game`'s runtime model genuinely its own shape rather than a
subset:

- **Some runtime data is derived, not raw.** Per-tile movement cost, for
  example, is a function of terrain/feature/elevation that `game` needs for
  pathfinding. Computing it once at translation and storing it flat avoids
  recomputing it from raw fields on every pathfinding step.
- **It must merge with mutable runtime state.** The runtime tile-state grids
  (`tile_owner`, `tile_occupant`, `improvement`; see the runtime tile-model
  ADR) are runtime concepts that do not exist in `WorldData`. `game`'s map
  representation is therefore necessarily its own thing — static terrain plus
  mutable grids — with `WorldData` as one input to the static half.

## Decision

Introduce an anti-corruption layer between `worldgen` and `game`. `game`
owns its own static-terrain model, shaped by runtime need, and does not
depend on `WorldData`'s shape at runtime.

- `game` calls `worldgen` to generate the world, then **translates**
  `WorldData` into its own static-terrain model at world setup. The model
  holds the player-impacting static terrain (elevation tier, biome,
  features, resources, start positions) plus runtime derivations computed
  once at translation (e.g. movement cost). Raw continuous height is **kept**
  on the `game` side, because elevation is mechanically and visually
  meaningful at runtime; only genuinely generation-internal fields stay
  behind the boundary.
- `WorldData` is a **transient setup input**, consumed by the translation
  step and discarded. After setup, no long-lived `WorldData` resource and no
  dependency on its shape survive in `game`'s runtime. This corrects the
  earlier intent to hold `WorldData` itself as the long-lived runtime
  resource.
- The static-terrain model is the static half of the runtime world model; it
  sits beside the mutable runtime grids.
- Shared vocabulary crosses the boundary **as itself**, without translation
  or duplication: `Coord` and the content registry handles (`ResourceId`,
  `Biome`) live in shared crates (see the content-registry ADR), so only
  truly `worldgen`-internal concepts stay behind the wall.
- The renderer bakes terrain chunks from `game`'s static-terrain model, not
  from `WorldData`.

## Consequences

**Positive:**

- `worldgen` and `game` evolve their data models independently. Generation
  changes that do not alter player-impacting output do not touch `game`.
- `game`'s runtime model is shaped by what it actually needs, including
  precomputed derivations, rather than inheriting generation's shape.
- `WorldData` does not linger past setup, in memory or in the dependency
  graph.
- The translation tax is small, because the shared crates already carved out
  the legitimately-common types; the boundary does not force duplicating
  `ResourceId` or `Coord`.

**Negative:**

- A translation step must be written and maintained; new player-impacting
  fields must be added in two places (the generation output and the
  translation into `game`'s model).
- Some Stage 4 work that read `WorldData` as a long-lived resource is redone
  to read `game`'s model instead.

## Alternatives considered

- **Keep consuming `WorldData` directly (status quo):** rejected. It couples
  `game` to `worldgen`'s internal representation and prevents either model
  from evolving freely. The coupling, not the memory, is the problem.
- **Define `game`'s model as a trimmed subset of `WorldData`:** rejected. A
  subset is still `worldgen`'s shape and re-couples on the next convenient
  field; it also cannot express runtime derivations or the merge with mutable
  state, which are exactly why `game`'s model differs in kind, not just size.
- **Hold `WorldData` as a long-lived runtime resource, ignore unused fields:**
  rejected. This is the memory-only framing; it leaves the coupling fully
  intact, which is the actual concern.

## References

- Documentation guide §4.4 (ADR format)
- ADR: runtime tile model (dense grids; the mutable runtime grids this model
  sits beside)
- ADR: content registry (the shared `ResourceId`/`Biome` handles that cross
  the boundary without duplication)
- Implementation plan, Stage 4.5 (anti-corruption layer)
- `documents/architecture/architecture.md` (the `worldgen`→`game` translation
  boundary)
