# ADR-0023: Runtime tile model — dense grids, not entities

**Status:** Accepted
**Stage:** 4 (Runtime world model)

---

## Context

The game needs a mutable, per-tile runtime representation of the world — who
owns each tile, what unit occupies it, what improvement is built on it — that
is correct, scalable, and determinism-preserving.

Two approaches were considered:

**Option A — Per-tile ECS entities.** Each tile is a Bevy `Entity` with
components for owner, occupant, improvement, and so on. State changes go
through normal component mutation.

**Option B — Dense grids (parallel `Field<T>` arrays).** Dynamic per-tile
state lives in a single resource as a set of parallel grids sized for the
whole map at world load.

---

## Decision

Dense grids (Option B).

The `TileState` resource holds five parallel `Field<T>` grids:

| Field | Type | Populated by |
|---|---|---|
| `tile_owner` | `Field<Option<NetworkId>>` | `FoundCity`, territory changes (Stage 7+) |
| `tile_occupant` | `Field<Option<NetworkId>>` | `MoveUnit`, `FoundCity` (Stage 6+) |
| `improvement` | `Field<Option<ContentHandle>>` | worker commands (Stage 8+) |
| `infrastructure` | `Field<u8>` | worker commands (Stage 8+) |
| `cleared` | `Field<bool>` | clearing worker commands (Stage 8+) |

**At most one player-built improvement per tile is a deliberate design
constraint**, not an incidental simplification. It eliminates per-tile
interaction resolution, keeps the grid model tractable, and is consistent with
the game's design.

---

## Rationale

### Mid-to-late-game scaling

A developed map has most of its land altered. Per-tile entities pay archetype
churn and Bevy change-detection cost in exactly that state, for no benefit:
tiles are accessed by coordinate and neighbourhood (coordinate math), not
iterated as a query. The dense-grid model makes altered and unaltered tiles
cost identically — building an improvement writes one value, with no
allocation, no archetype move, and no per-entity overhead.

### Memory footprint

Whole-map state for the default 80×80 map is on the order of ~1 MB total
across all grids, contiguous and cache-friendly. This is flat regardless of
how developed the map becomes.

### Yield computation

The `tile_yield(coord)` function reads both static `WorldData` and dynamic
`TileState` by coordinate. This read pattern is trivially satisfied by dense
grids and would require either an entity query (slow for neighbourhood reads)
or a secondary spatial index (extra bookkeeping) with a per-entity approach.

### Reaction path

Change-detection (`Changed<T>`) on tiles is not the game's reaction path.
All state changes flow through the command layer and are announced via
buffered events (`UnitMoved`, `CityFounded`, `TileImproved`). Giving up
`Changed<T>` on tiles costs nothing; the command layer already provides the
notification.

### Determinism

Per-tile entities have `Entity` indices that diverge across machines. Dense
grids have no entity identity — they are addressed by `Coord`, which is the
same on all machines. This keeps the authoritative tile model
determinism-preserving without any extra machinery.

---

## Cross-references

- ADR-0024: explains why cross-tile references use `NetworkId`, not `Entity`.
- `crates/game/src/sim/world_model.rs`: the `TileState` resource implementation.
- `documents/planning/implementation_plan2.md` §Stage 4: tile model decision noted.
