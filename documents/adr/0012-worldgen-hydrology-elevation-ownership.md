# ADR-0012: Hydrology Elevation Ownership and Canyon Wall Impassability

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

The hydrology stage (Stage 3.3) modifies elevation data in two places:
sink resolution raises some tiles to their basin spill point elevation,
and canyon carving lowers some tiles by one or two height levels. Both
operations produce a `height_level` field that differs from the one
produced by the tectonics stage.

This creates a structural question: who owns the authoritative
`height_level` field after hydrology runs? The stages downstream of
hydrology — biomes (3.4), resource placement (3.5), and starting
position selection (3.6) — all need to read the post-modification
field. If they read `TectonicData::height_level` directly, they get the
pre-hydrology values, which is wrong.

A secondary question arises from canyon carving specifically: canyon
walls are adjacent tiles marked as impassable features. The height delta
between a carved canyon floor and its wall tile is ≥ 2, which
intersects the cliff impassability rule established in design doc §1.4.
The relationship between canyon wall impassability and the general cliff
rule needs to be explicit.

## Decisions

### 1. Elevation ownership: `HydrologyData` owns a modified copy during development; production pipeline mutates in-place

During the active tuning period for Stages 3.3 and 3.4, `HydrologyData`
carries its own `height_level: Field<u8>` that is initialized as a
clone of `TectonicData::height_level` and then modified by sink filling
and canyon carving. `TectonicData::height_level` is left unmodified.

Downstream stages (3.4 onward) read `HydrologyData::height_level`, not
`TectonicData::height_level`.

This arrangement is explicitly temporary. Its purpose is to preserve
both the pre- and post-hydrology elevation fields as separate PNG
outputs during parameter tuning, making it possible to visualize what
changed and why. Once Stage 3.4 (biomes) is complete and the hydrology
parameters are stable, the clone is eliminated: the hydrology stage will
take `height_level` by mutable reference (or take ownership), modify it
in-place, and `TectonicData` will thereafter always carry the
post-hydrology values. `HydrologyData` will drop the field.

The cleanup is a single-PR refactor with no algorithm changes. The
trigger is the Stage 3.4 gate.

### 2. Canyon wall impassability: explicit application of the §1.4 rule

Design doc §1.4 establishes that a height delta ≥ 2 between adjacent
tiles is impassable by default. Canyon carving lowers a river tile by
one or two height levels while leaving the perpendicular neighbors at
their original elevation, producing exactly this delta.

Canyon walls are therefore impassable under the existing §1.4 rule —
no new rule is needed. `HydrologyData` marks perpendicular canyon
neighbors with a `is_canyon_wall: bool` feature flag. This flag:

- Serves as a rendering hint (canyon walls render differently from
  ordinary cliff edges).
- Serves as a gameplay annotation (pathfinding can check the flag
  directly rather than re-deriving impassability from height delta).
- Does **not** introduce a new impassability mechanism; it is an
  application of the existing delta ≥ 2 rule to a specific terrain
  feature.

The §1.4 blocker recorded in `STATUS.md` before Stage 3.3 is therefore
resolved by this explicit confirmation, not by a new design decision.

## Consequences

**Positive:**

- The debug-period duplication is intentional and documented, so it
  will not be mistaken for a permanent architecture or left in place
  accidentally.
- Downstream stages have a single, unambiguous source of truth for
  post-hydrology height levels: `HydrologyData::height_level`.
- Canyon wall impassability follows the existing rule without
  introducing a special case. Pathfinding and traversability logic
  written for the general delta ≥ 2 rule handles canyons correctly
  without modification.
- The `is_canyon_wall` flag makes canyon walls explicitly queryable,
  which is useful for rendering (canyon walls have a distinct visual
  treatment) and for future bridge/tunnel engineering mechanics.

**Negative:**

- The temporary clone adds one `Field<u8>` worth of memory (one byte
  per tile; 36 KB for a 240×150 map) to `HydrologyData` during the
  tuning period. This is negligible.
- The cleanup refactor at the Stage 3.4 gate is a required step, not
  optional. If it is deferred past Stage 3.4, downstream code will
  silently read the wrong field. The gate condition for Stage 3.4
  includes this cleanup as a prerequisite.
- Two separate fields (`TectonicData::height_level` and
  `HydrologyData::height_level`) with different semantics coexist
  temporarily. Code that reads the wrong one compiles silently.
  Addressed by naming convention: all post-Stage-3.3 code uses
  `world_data.hydrology.height_level`; any reference to
  `world_data.tectonics.height_level` in Stage 3.4+ code is a bug.

## Alternatives considered

**Mutate `TectonicData::height_level` in-place immediately** — have
the hydrology stage take a `&mut TectonicData` and modify the field
directly, with no duplication. Clean ownership, no cleanup needed.
Rejected for the tuning period because it eliminates the ability to
compare pre- and post-hydrology elevation in PNG outputs side by side.
This comparison is valuable during parameter sweeps: it shows which
tiles were sink-filled (raised) and which were canyon-carved (lowered),
making it easy to diagnose over-filling or over-carving. The cost is
one temporary field and one cleanup PR.

**Keep `TectonicData::height_level` as the canonical field; have
hydrology annotate a diff** — store only the modifications (a sparse
map of `Coord → new_height_level`) in `HydrologyData`, and have
downstream stages apply the diff before reading. More memory-efficient
for sparse modifications. Rejected because it pushes merge logic into
every downstream consumer, and because canyon carving is not sparse
on maps with many rivers — the diff could be nearly as large as the
full field.

**Add a `PostHydrologyData` wrapper that shadows `TectonicData`** —
a newtype or wrapper struct that presents a unified view and routes
field access to the correct source. Adds abstraction without reducing
the underlying duplication. Rejected as over-engineering for a
two-stage, known-temporary situation.

**Store canyon walls as a height delta ≥ 2 edge, not a tile flag** —
model canyon impassability as an edge property (like the future bridge
model) rather than a tile flag. Consistent with how bridge traversal
is designed. Rejected because the edge data model is explicitly deferred
until bridges are implemented (design doc §1.4), and introducing it
here to handle one feature type would force that deferred decision
prematurely.

## Cleanup condition

When the Stage 3.4 (biomes) gate is reached:

1. Remove `height_level: Field<u8>` from `HydrologyData`.
2. Change the hydrology stage entry point to accept `&mut TectonicData`
   (or take ownership) and modify `height_level` in-place.
3. Update all downstream stage code that reads `hydrology.height_level`
   to read `tectonics.height_level` instead.
4. Remove the pre-hydrology height level PNG output from `mapdump` and
   `maptest` (or rename it to reflect that it is now identical to the
   post-hydrology output, making it redundant).
5. Write a brief follow-up note in this ADR's status — change
   `Status: Accepted` to `Status: Partially superseded — cleanup
   complete` with a date. No new ADR is needed for this cleanup; it
   is the planned resolution of a known temporary state.

## References

- Design doc §1.4 — height levels and cliff impassability rule
- Design doc §1.11 — hydrology stage, canyon carving algorithm
- ADR-0011 — hydrology algorithm choices (companion ADR)
- `crates/worldgen/src/world_data.rs` — `HydrologyData` struct
- `crates/worldgen/src/stages/hydrology/canyon.rs` — carving
  implementation
- Stage 3.4 gate condition — cleanup trigger
