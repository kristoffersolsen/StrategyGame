# ADR-0024: NetworkId as the authoritative cross-tile reference

**Status:** Accepted
**Stage:** 4 (Runtime world model)

---

## Context

The `TileState` dense grids (ADR-0023) store references to game actors: which
city owns a tile, which unit occupies it. These references must identify the
same actor on every machine in a multiplayer session and across save-load
cycles.

Bevy `Entity` indices are a natural candidate — they are cheap, copy-safe
integers already assigned to every entity. But they have a critical property
that makes them unsuitable as authoritative references: they reflect the order
in which entities were *spawned locally*. Two machines running the same
command sequence can produce different `Entity` indices for the same logical
actors, depending on startup order, deferred entity creation, and Bevy
internal bookkeeping.

---

## Decision

Cross-tile references in `TileState` are `NetworkId`, not `Entity`.

```rust
pub(crate) struct NetworkId(u64);
```

`NetworkId` is a monotonic counter advanced exclusively in
command-processing order by `NetworkIdAllocator`. A non-authoritative
`NetworkId → Entity` lookup cache (`EntityLookup`) is permitted as a
convenience. It is:

- **rebuilt** from authoritative state after command processing;
- **never serialised** or sent over the network;
- **never used** to drive game logic (only to find a rendering entity
  from a known network id).

The rule is identical to the `Handle`-vs-string-id discipline in ADR-0022:
a stable authoritative identity (`NetworkId`), a derived runtime index
(`Entity`), and a hard rule that only the former crosses saves and the wire.

---

## Rationale

### The `Entity` divergence problem

When `MoveUnit(actor_id, destination)` is processed, the command processor
writes `Some(actor_id)` into `TileState::tile_occupant[destination]`. If
`actor_id` were an `Entity`, two machines might store *different* values for
the same logical actor. Replaying the command log to reconstruct state would
produce divergent grids. `NetworkId` is derived from the command sequence
itself — the same commands always produce the same ids — so replayed state is
identical.

### Allocation discipline

`NetworkIdAllocator::next()` must only be called from the command processor,
in the order commands are drained from the queue. The allocation order *is*
the determinism guarantee. Calling it from arbitrary systems (e.g., spawning
logic) would couple allocation order to system scheduling order and break the
guarantee.

### Lookup cache design

`EntityLookup` is populated after command processing, once the authoritative
`NetworkId` is known and the entity has been spawned. It is appropriate for:
- Rendering systems finding the mesh entity to update when `UnitMoved` fires.
- UI systems highlighting a selected actor.

It is not appropriate for:
- Serialisation (the cache is transient; only `NetworkId` is stable).
- Network synchronisation (the cache would diverge across machines).
- Game logic decisions (use `NetworkId` lookups into `TileState` instead).

---

## Alternatives considered

**Using a secondary HashMap inside each tile struct.** Rejected: tiles are
not structs; they are parallel-array slots. There is no "tile struct" to hang
a map from.

**Using `Entity` with a coordination protocol to keep them in sync.**
Rejected: this is complex, fragile, and undermines the entire point of the
command-layer architecture, which is that the simulation on each machine
advances identically from identical commands with no coordination required.

---

## Cross-references

- ADR-0022: the same stable-id / derived-index pattern applied to content
  handles vs. string IDs.
- ADR-0023: the tile model that stores `NetworkId` references.
- `crates/game/src/sim/network_id.rs`: `NetworkId`, `NetworkIdAllocator`,
  `EntityLookup` implementations.
