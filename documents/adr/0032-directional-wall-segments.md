# ADR 0032: One-sided owned wall segments — `(tile, facing)` state on a composed tile-edge defense

**Status:** Proposed
**Date:** 2026-06-22
**Deciders:** Kristoffer

## Context

Cities had no defensive mechanic, which makes early aggression a near-free win and
leaves the settlement layer (`cities.md` §2.12–§2.13) without a defensive
counterweight. The Ancient era needs simple walls. Several things had to be decided
together: where walls live in the data model, how they are built, what they cost, how
they compose with the terrain the map already produces, and how they feed combat —
while **siege mechanics are deliberately deferred to later eras**, so Ancient walls
ship without a designed counter.

An earlier revision of this ADR modeled walls as **per-edge state**, reusing a
supposed road edge-state primitive. That premise no longer holds: roads are **tile**
infrastructure, not edge state (ADR-0035), so there is no road edge primitive to
reuse, and the engine grows no general edge primitive. More fundamentally, modeling a
wall as a *shared edge* mis-describes it — a wall is one-sided and owned. This ADR
reverses the per-edge model accordingly.

## Decision

**Walls are one-sided owned `(tile, facing)` segments — not edge state.**

- **Representation.** A wall segment is owned state keyed to a builder-owned **tile**
  and an outward **direction**, held in a settlement-side defense store (read by
  combat, never by the per-turn yield sweep). It is not an edge primitive: an edge
  models a neutral object *shared* by two tiles, but a wall is built on *your* tile
  facing *out* — the far side has no claim on it, and if the enemy walls their side
  that is a second, separately owned segment. The one-sided owned model is therefore
  the *correct* primitive, and it removes the shared-edge double-ownership problem at
  contested borders (each segment lives on its own owner's tile; there is no second
  copy to arbitrate or keep in sync). Segments sit on the outward faces of
  **district tiles** (the built-up perimeter), not arbitrary claimed-tile faces.

- **Construction.** A wall segment is a `WallSegment` build target in the job system
  (ADR-0034), not a job-slot building and not a tile improvement. By the two-axis rule
  (ADR-0030) it is **build-slot work on a city's own tiles** and **worker work on an
  outpost's**; it holds no job-slots and consumes no permanent district capacity, so
  it never touches the staffing economy.

- **Cost.** Worker-time (or build-slot time) **plus a modest base-materials (lumber)
  charge per segment**. This is the deliberate exception to the worker-time-only
  improvement rule (`workers.md` §5): a materials cost makes walling a whole perimeter
  a real guns-vs-walls choice rather than free work for idle labor. For worker-built
  segments the materials come through the worker's materials buffer (ADR-0035); for
  build-slot-built city segments, through the build-slot draw — both from the network
  materials pool.

- **Composed defense, not a shared edge model.** When a unit attacks into a tile, the
  defender's modifier is **composed at resolution time** from two independent sources:
  the natural edge classification of the crossed boundary (cliffs via `edge_class`
  ADR-0012, rivers, coast) and the defender tile's **outward wall segment toward the
  attacker**, if any. "Back the settlement onto a cliff and wall only the open sides"
  still falls out — now from combat composing two representations rather than from one
  shared edge store. Combat reads exactly one wall segment per assault: the target
  tile's segment facing the attacker (`combat.md` §3.10).

- **Ancient walls are wooden.** A defender bonus across the walled face; **no
  damage/destruction model** (a segment is static until its tile is captured); **no
  siege equipment** to counter them. Multiple layers are allowed and need no special
  rule — an attacker crossing successive walled district-tile faces pays the modifier
  at each ring, the "sieges are an investment" property emerging from one mechanism.

## Consequences

**Positive:**

- The `(tile, facing)` model matches what a wall *is* (one-sided, owned) and needs no
  edge primitive — consistent with the all-tile representation (ADR-0023, ADR-0035).
- Contested borders are trouble-free: each side owns its own segments on its own
  tiles; nothing is shared, so nothing must be arbitrated or de-duplicated.
- Routing walls through the job system as a no-slot build target keeps them off the
  staffing economy while reusing one construction path (ADR-0034).
- Composing natural and built defense at resolution time keeps "use natural blockers"
  a real mechanic without forcing walls and terrain into one representation.

**Negative:**

- "Wall this face" as a player action must expand into the set of perimeter
  `(tile, facing)` segments; the stored primitive is atomic, the UI verb is not.
  Accepted as a UI concern.
- A multi-tile footprint's perimeter is several segments; unambiguous (each is a
  specific tile + direction) but more book-entries than a single "settlement is
  walled" flag.
- Ancient walls ship without a counter (siege deferred); accepted and flagged.

## Alternatives considered

- **Per-edge wall state on a shared edge-defense model** (this ADR's prior revision):
  rejected. It presumed a road edge primitive that no longer exists (roads are tile
  state, ADR-0035), and it mis-models a one-sided owned thing as a shared edge,
  creating a double-ownership problem at contested borders.
- **Walls as a district building (a job-slot):** rejected. Walls are perimeter faces,
  not tile-interior infrastructure, and would consume building capacity and staffing
  for something that produces no yield.
- **A single "settlement is walled" flag (uniform defense):** rejected. It removes the
  directional choice — wall the open sides, lean on the cliff — that makes terrain and
  walls interact, and the visual/strategic texture of partial walls.
- **Worker-time-only walls (no materials):** rejected as the primary model. It makes
  full walling a no-brainer for idle labor and removes the guns-vs-walls tradeoff; the
  materials cost is the point.

## References

- ADR-0012 (cliffs / `edge_class` — the natural edge classification combat composes
  with walls).
- ADR-0023 (dense tile state; the all-tile representation walls fit).
- ADR-0030 (two-axis construction rule: city = build-slot, outpost = worker).
- ADR-0034 (job system; `WallSegment` as a no-slot build target).
- ADR-0035 (roads are tile state, not edge — why no edge primitive exists to reuse;
  worker materials buffer funds worker-built segments).
- `combat.md` §3.10 (per-assault defense composition — **pending sync** to the
  composed, non-shared-edge model), §5.4 (unit-strength validation).
- `workers.md` §3, §5 (wall build job, materials-cost exception — **pending sync**
  away from the edge framing); `ancient-content.md` §6 (walls in the Ancient set).
- Design pillars 0.1 (realism backdrop), 0.5 (no over-engineering, reuse).
