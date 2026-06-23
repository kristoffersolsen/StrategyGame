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

**Construction model.** A **city center can build** within its own footprint (its
bootstrap slot, plus build-slots in any production buildings it constructs,
ADR-0028); an **outpost center cannot build at all**. **Everything outside a city is
built by workers** (`workers.md` §1, §3): a worker **places a district footprint and
constructs the buildings on it**, and likewise builds improvements, roads, bridges,
and walls on non-city tiles. The worker is the player's single means of changing
tiles beyond a city's footprint.

Building construction still **draws network materials**, demand-first at the build
site, so an outpost unreachable by road can have its tiles improved (worker-time,
little or no material cost) but **cannot have buildings constructed** — the
support-from-a-city dependency that makes distant settling hard and roads worth
building (`cities.md` §2.9) is preserved, now carried by the **materials
requirement** rather than by the labor source.

This is **not** a second construction primitive (pillar 0.5). Build-slots and
worker-time are two **labor inputs to one progress machine**: a build job accumulates
invested materials against a cost and flips to complete, regardless of which labor
advanced it; and worker-time is **already** the labor for improvements and roads, so
extending it to outpost buildings reuses that channel rather than duplicating the
build-slot system. This **supersedes** the `workers.md` §1 scope that said workers do
not place districts — workers now place footprints **and** construct their buildings.

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
- Workers are the single interaction model for everything outside a city, so there
  is no per-outpost build queue **and** no centralized remote-build queue mixing a
  city's own work with its outposts'. Construction draws network materials
  demand-first at the build site, so no separate materials-transfer system is needed
  (pillar 0.5).

**Negative:**

- A network in deficit stores nothing, so silos sit empty; this can read as "storage
  broken" without a clear UI signal (flagged for the UI pass).
- Two coexisting scope models (empire-wide currency, network-local goods) are more
  to hold in mind, though they map onto an intuitive physical distinction.
- Demand-first plus nearest-first surplus routing must be proven order-independent
  for determinism (pillar 0.4).
- Worker-time becomes the single bottleneck for **all** exterior development (outpost
  buildings, improvements, roads), so worker create/upkeep cost and throughput become
  the load-bearing tuning lever for the whole outside-the-city economy. The UI win
  over a centralized build queue holds **only if** worker tasking is intent-level (a
  "develop this outpost" directive the worker sequences itself, §0.6); per-building
  worker micro would relocate the management burden onto the map rather than removing
  it.

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
- **Road-connected city constructs outpost buildings** (the originally-chosen model:
  workers place footprints, and a connected city's build capacity constructs the
  buildings over the network): rejected on reconsideration. It centralizes every
  outpost's construction into the owning city's own build queue — a capital with
  several outposts holds one long queue mixing its own buildings with all its
  outposts', serialized through the capital's build-slot throughput — and it presents
  a confusing two-agent seam (a worker places the footprint, then a distant city
  reaches out to build on it). Routing all exterior construction through workers is
  one legible rule. The original pillar-0.5 objection (worker-built buildings as a
  parallel construction path) is answered by the unified one-progress-machine model
  in the Decision; the original materials-transfer objection does not arise, because
  construction is a demand-first draw on network storage at the build site, not a
  background transfer.
