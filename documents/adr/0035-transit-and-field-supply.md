# ADR 0035: Transit route layer and field supply — tile-based roads, derived paths, range-limited field supply, worker materials buffer

**Status:** Proposed
**Date:** 2026-06-22
**Deciders:** Kristoffer

## Context

ADR-0030 established roads as the supply-network graph but left their runtime shape,
their build/queue model, and their cost loosely specified, and `workers.md` §9
described them as *per-edge* state. Designing the worker forced these closed, and
forced a related question ADR-0030 deferred: **what does building roads cost**, so the
map is not trivially blanketed in them, and how does that cost scale with distance
from the player's developed core?

It also forced a representation decision the project had been drifting on: are roads
(and rivers, and bridges) edge state or tile state? The tile model (ADR-0023) is
per-*tile*; an edge primitive would be new machinery. And the supply system has two
distinct notions that were being conflated — goods distribution between settlements
(ADR-0030, connectivity-wide) and the sustainment of a *unit in the field* (combat.md
§3.11, range-limited).

Constraints: pillar 0.4 (deterministic pathing/tiebreaks), pillar 0.1 (logistics
physically matter), pillar 0.5 (reuse §3.11 rather than invent a cost system), and
player legibility (the §3.11 foraging state was judged too much to expose in the
Ancient era).

## Decision

**Roads, rivers, and bridges are tile state, not edge state.** A road is tile
infrastructure (`infrastructure: Field<u8>`, ADR-0023); rivers are a tile property;
bridges (Classical+) are tile infrastructure on river tiles. This supersedes the
per-edge framing in `workers.md` §9. The engine grows **no** edge primitive for
routes. (The only owned directional state in the design is walls, which are
`(tile, facing)` segments, not a shared edge — ADR-0032.)

**Transit is its own module/system,** the route layer (roads now; rail, sea, air in
later eras), owning route jobs and the connectivity and field-supply-range queries
the supply network and worker resupply read. A route is not a settlement build target;
it belongs to no settlement and may connect arbitrary tiles, including a stray pass
road built for movement rather than trade.

**Road jobs store only endpoints.** A road job is `{ from, to }`; the tile path is
**never stored** — it is derived on demand by deterministic A\* that prefers existing
roads, so already-laid road tiles (which live in tile state) are reused and only the
gaps remain. The path is disposable and auto-reroutes when the network changes; a
resuming worker re-derives the same continuation. Road jobs live in a **per-player,
empire-wide `RoadPool`**, claimed by any road-assigned worker (nearest unclaimed
first, one per job — ADR-0034). They share only the `Available`/`InProgress` lifecycle
with settlement jobs, deliberately not unified (ADR-0034).

**Field supply is range-limited and roads extend it.** A unit in the field is in
supply when within a finite range of a **supply source** (a city, or a relay outpost
holding stock) measured along connected roads; technology increases the range. Roads
extend reach, but not infinitely — beyond range a unit is out of supply even on a
road. Range **re-anchors at any source**, so reach is projected by chaining **trade
outposts**, each within range of the previous, which is the intended cost of
distance. This **field-supply** layer (range-limited, for units in the field) is
distinct from ADR-0030 **settlement-goods** distribution (connectivity-wide, between
settlement storages); the two coexist and must be held apart.

**Worker construction cost is its §3.11 materials buffer.** A worker's supply category
is **building materials** — its "ammunition." Building consumes materials from the
worker's buffer; in field-supply range the buffer refills straight from network
materials, demand-first, exactly as build-slot construction draws them; out of range
the worker builds from its carried reserve until empty, then retreats to refill. One
buffer thus carries both the **cost** (materials drawn from the network pool — so
worker construction competes with build-slot construction for the same materials, and
road/outpost-spam is self-limiting through the economy) and the **friction** (the
shuttle when far from the network). Materials is the *only* build-relevant buffer;
food is ambient unit upkeep (§3.11), not a second per-worker pressure. Buffer *size*
is the tuning lever for how far past the network a worker can build before retreating.

**Foraging is deferred, not deleted.** The Ancient era surfaces a single slow
out-of-supply rate and no "foraging" jargon — everything pre-gunpowder forages, so
there is no contrast to expose. §3.11's model is unchanged; the faster can't-forage
rate appears when industrial units arrive, which is what produces the medieval-vs-
modern mobility asymmetry. Ancient simplifies the *presentation*, not the mechanic.

## Consequences

**Positive:**

- Roads drop into the existing dense tile model with no new edge machinery, and the
  road graph is exactly the connected-component graph the supply network wants.
- Endpoint-only road jobs are leanest possible: no stored path to keep in sync,
  automatic reroute, deterministic resume — the denormalization-free outcome.
- The materials buffer makes roads cost something real (network materials + shuttle
  time) via reused §3.11 mechanics, with no new resource and no auto-gate, and ties
  road projection to settlement placement (trade-outpost chains).
- The field-supply / settlement-goods split removes a latent contradiction (a finite
  unit-supply range coexisting with connectivity-wide goods).
- A `transit` module gives the route layer a home whose name does not lie, with room
  for later modalities.

**Negative:**

- Field supply and settlement goods are two range models to hold in mind; mitigated by
  their clean physical distinction (a depot's reach vs. a trade route's existence).
- Worker refill couples to a transit range query each turn; cheap, but a real
  cross-module dependency (worker → transit).
- Endpoint-only roads can thrash (build, reroute, leave stray segments) when the
  network changes mid-build; bounded because A\* strongly prefers already-built roads,
  and stray road tiles are harmless permanent state.

## Alternatives considered

- **Roads as edge state** (the `workers.md` §9 framing): rejected. It needs an edge
  primitive the tile model lacks, for a thing that is natively tile-shaped and whose
  connected-component graph the supply system already wants per tile.
- **Storing the road path (or completed-tile set) on the job:** rejected. The built
  tiles already live in tile state and A\* reuses them, so storing them duplicates
  state that can drift — keep the job to its endpoints.
- **Worker-owned road queues:** rejected (ADR-0034) — a long road would die with the
  worker; site-ownership keeps progress.
- **Infinite road supply range / range from cities only:** rejected. Infinite range
  makes logistics ignorable; city-only range makes relay outposts unable to relay, so
  trade-outpost chains (the intended distance cost) would not work.
- **Action-drain on an abstract effectiveness buffer:** rejected for the materials
  buffer — action-drain is wrong for a positional effectiveness stat but correct for a
  literal consumable, and materials are literal.
- **Keeping foraging as an exposed Ancient state:** rejected as complexity with no
  contrast unit in the era; deferred to when industrial units create the asymmetry.

## References

- ADR-0023 (dense tile state; `infrastructure` field).
- ADR-0030 (supply networks: connectivity, settlement-goods distribution, no-refund
  severance — the layer field supply is distinct from).
- ADR-0032 (walls as `(tile, facing)` — the only owned directional state).
- ADR-0034 (job lifecycle shared with road jobs; one-per-job; deliberately not
  unified).
- ADR-0012 (`edge_class` cliffs — terrain edge classification, unrelated to routes).
- `combat.md` §3.11 (field-supply states, the buffer model reused), §3.13 (unit
  upkeep); `workers.md` §3, §5, §9 (worker road/wall jobs, materials-cost, the
  superseded edge framing); `resources.md` §9 (settlement-goods distribution);
  `cities.md` §2.9 (roads/regions).
- Design pillars 0.1 (realism backdrop), 0.4 (determinism), 0.5 (reuse, no
  over-engineering), 0.6 (intent-level worker tasking).
