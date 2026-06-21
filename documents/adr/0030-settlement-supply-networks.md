# ADR 0030: Settlement supply networks — roads, network-scoped goods, settlement-local storage, demand-first distribution

**Status:** Proposed
**Date:** 2026-06-20
**Deciders:** Kristoffer

## Context

`cities.md` §2.7 left the scope of building materials and food open ("city-local or
per-region"), while `resources.md` §6 fixed gold as empire-wide. Enumerating the
Ancient content forced this open question closed, because it determines how outposts
are built, how supply severance behaves, and what the Ancient logistics layer is.

Several decisions had to be made together: the scope of bulk goods; where they are
stored versus where they can be spent; how surplus accumulates; what happens when
the network is cut; how an outpost (whose center cannot build, ADR-0028) gets its
buildings constructed; and what the Ancient era's first infrastructure is. The
constraints were pillar 0.4 (deterministic distribution and tie-breaking), pillar
0.5 (no mechanism duplicating another), pillar 0.1 (supply lines should physically
matter), and the design's preference for severance to be a real strategic event
rather than cosmetic.

## Decision

**Roads are a worker-built infrastructure job** (`workers.md` §1, §3) and form the
**explicit graph** of the supply network from the Ancient era. A settlement's reach
is its road-connected component.

**Scope:** building materials and food are **network-local**; gold, science, and
culture are **empire-wide**. Stock and reach are separated:

- **Stock** lives in **settlement-local storage** — the center has base capacity; a
  **silo** building (any district) adds more. Each settlement physically holds its
  own pile (severance-safe by construction).
- **Reach:** at spend time the network **sums the stock of all road-connected
  settlements** and draws from it. Goods move only when consumed, only over live
  roads — no background transfer.

**Distribution is demand-first, then surplus to storage.** Each turn: (1) **live
network demand** — population growth-flow and unit food upkeep — is consumed
**immediately and never stored**; then (2) **surplus** flows to storage
**nearest-first** (deterministic tiebreak) into connected silos with capacity.
Storage is **downstream of consumption**: a settlement's own production is not
privileged for its own silos. Overflow past all reachable capacity is **lost**.

**Food has two roles:** the growth-driving flow (immediate, unstored) and the
surplus (stored). **Unit upkeep is food** (Ancient era), drawn demand-first from the
unit's network, reducible by the barracks; this ties army size to the agricultural
base.

**Severance** splits the network into independent components; each keeps the stock
its silos hold and **re-runs demand-first** on its smaller network. In-progress
construction on a stranded side **pauses** (it does not fail or refund).

**Construction model:** a **city center can build** (bootstrap slot + production
buildings, ADR-0028); an **outpost center cannot**. Workers **place** outpost
district footprints, and a **road-connected city's build capacity constructs** the
buildings, drawing the network's materials. An outpost with no road to a city can
have tiles improved (worker-time only) but cannot have districts/buildings
constructed — the support-from-a-city dependency that makes distant settling hard.
This **supersedes** the `workers.md` §1 scope that said workers do not place
districts.

## Consequences

**Positive:**

- Separating stock from reach dissolves the "who gets the pool on a split?" problem:
  there is no pool, only per-settlement piles viewed in aggregate, so severance is a
  recalculation, not a tug-of-war.
- Demand-first prevents a downstream consumer (a city fed by a farm outpost) from
  being starved when the supplier builds a silo: demand is met before anything is
  stored, so a silo captures only true surplus.
- Settlement-local storage gives the silo a real purpose (buffering against
  severance) and makes capacity a genuine decision.
- Roads make supply lines physically matter from the first era (pillar 0.1) and give
  raiding/chokepoint play an early payoff.
- Outpost construction via a connected city needs no per-outpost build queue and no
  early materials-transfer system (pillar 0.5).

**Negative:**

- A network in deficit stores nothing, so silos sit empty; this can read as "storage
  broken" without a clear UI signal (flagged for the UI pass).
- Two coexisting scope models (empire-wide currency, network-local goods) are more
  to hold in mind, though they map onto an intuitive physical distinction.
- Demand-first plus nearest-first surplus routing must be proven order-independent
  for determinism (pillar 0.4).

## Alternatives considered

- **Even-fill** (resources accumulate evenly across all non-empty storage): an
  empire-wide pool in disguise — it teleports a frontier mine's output to the
  capital and nullifies severance. Rejected.
- **Fill-local-first** (a settlement fills its own silos before exporting): starves
  downstream consumers; a silo in a feeder outpost would stop it feeding the city it
  supplies. Rejected.
- **Empire-wide materials/food storage:** defeats the purpose of the network and
  makes severance consequence-free. Rejected.
- **Proximity regions + remote construction** (no explicit roads; build anywhere in
  a proximity region): rejected as blurry — "am I in supply?" becomes a fuzzy
  distance check rather than a legible yes/no over explicit edges.
- **Workers construct buildings directly:** rejected — a parallel construction path
  (against ADR-0028) forcing an early materials-transfer system; routing
  construction through the network's existing build capacity avoids both.

## References

- `resources.md` §9 (storage, distribution, upkeep), §6 (gold scope).
- `cities.md` §2.7 (scope), §2.9 (roads/regions), §2.12 (construction model).
- `workers.md` §1, §3 (district siting, roads; the superseded scope).
- `combat.md` §3.11 (field-supply states), §3.13 (unit food upkeep).
- ADR-0028 (build capacity; outpost centers cannot build).
- Design pillars 0.1 (realism backdrop), 0.4 (determinism), 0.5 (no
  over-engineering), 0.6 (deferred player-controlled distribution priority).
