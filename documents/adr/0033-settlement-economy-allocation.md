# ADR 0033: Settlement economy runtime — owned-data record and priority-driven population allocation

**Status:** Proposed
**Date:** 2026-06-22
**Deciders:** Kristoffer

## Context

ADR-0028 settled the staffed-work *model* (job-slots and build-slots, district-owned
and building-held capacity, co-location). Two things it left open were forced by
designing the runtime: **how the sub-settlement structure is represented** (entities
or data), and **how population is allocated** across slots to serve the player's
intent without per-slot micromanagement.

ADR-0023 settled the *tile* layer (dense grids; "only units and cities are
entities") but not the structure *inside* a settlement — a district is not a tile,
and is also not a unit or city. The conventional instincts (a Bevy entity per
district and per building; a manual per-slot staffing UI; absolute per-yield
production targets) were each challenged: per-entity sub-structure fits ECS poorly
(it is reached *through* its owner, not iterated as a population), and manual
staffing plus absolute targets are micromanagement that pillar 0.6 rejects.

Constraints: pillar 0.4 (deterministic allocation and tiebreaks — the allocator runs
server-authoritatively and feeds replay/tests), pillar 0.6 (intent-level control),
pillar 0.5 (no over-engineering — no accelerators before they are needed), and the
ADR-0023 entity discipline.

## Decision

**Representation — owned data, not entities.** A settlement is the entity (city and
outpost unified, tier derived from the center handle). Its districts, buildings, and
claimed-tile improvements are **owned data on the settlement** — a nested record
(`districts: Vec<District>` → `buildings: Vec<Building>`) plus a `claimed: Vec<Coord>`
index — never entities. This follows ADR-0023's rule directly: these are reached
*through their owner* during a per-settlement economic tick and mutated at human pace,
so owned data avoids the archetype churn, per-instance `NetworkId` shadow, and
relation bookkeeping that entities would impose for no iteration benefit. `struct` is
not `Entity`: grouping fields is free; runtime identity in archetype tables is the
narrow thing entities are *for*, and sub-settlement structure does not need it.

**Authoritative vs. derived.** The nested record is authoritative for the economy.
Tile ownership and improvement *identity* stay authoritative on the dense grid
(ADR-0023). The settlement's `claimed` list and the tile-side district index are
**derived**, written by the same command processor that mutates the record (single
writer), so they cannot drift. "Is this tile improved" is answered by reading the
grid, never by a field duplicated on the settlement.

**Allocation — priority ratios, greedy with bundles.** The player sets a
**priority-ratio goal** (integer weights per yield summing to a fixed total), not
absolute targets — a ratio is legible and has no "above achievable maximum"
awkwardness. Population is assigned by a **greedy allocator with building-completion
bundles**: candidate moves are *staff one slot* and *staff a building's remaining
empty slots as a bundle*, the bundle being what captures a **building-level
completion bonus** (the one non-separable element; district-level completion was
rejected so the bundle stays a small constant at any settlement size). Each candidate
scores as priority-weighted marginal yield measured against the running per-axis
total, so value declines as an axis accumulates and the realized distribution
converges on the priority ratios. For **network-scoped goods** (food, materials) the
saturation is measured against the *network's* demand-plus-storage headroom, not local
stock (ADR-0035, ADR-0030) — a settlement does not pivot off food while its network
starves. Deterministic by construction: integer/permille math, ratios compared by
cross-multiplication, ties broken by ascending slot index. The allocator is a **pure
function**, reusable as the inner loop of the AI strategist's goal search.

**Build-slot population split — off the top, player-throttled, no auto gate.**
Build-slots claim population **off the top** before the yield allocator runs on the
remainder — building a production building *is* the instruction to staff it. The
player throttles by disabling a building (`enabled = false`, which removes its slots
from allocation), the mirror of the priority vector for the build channel. There is
**no automatic material gate**: build-slots draw network materials freely, and
draining the stockpile is a player-owned, UI-warned failure state. Auto-gating was
rejected because it would make supply disruption painless and neuter the strategic
stakes of the road network (ADR-0030).

**Deferred accelerators.** Two optimizations live behind the allocator's pure
interface and are *not* built now: a priority queue (O(S log S), justified only when
the AI goal-search makes the allocator a hot inner loop) and a local-improvement pass
(to close greedy's threshold gap on small settlements, justified only if playtesting
shows manual placement beats auto by a tempting margin). Building either before its
trigger is premature (pillar 0.5).

## Consequences

**Positive:**

- Owned data keeps the per-settlement economic tick a cache-friendly walk with no
  archetype or relation cost, while leaving cross-settlement work (the real
  parallelism surface) independent per entity.
- Priority ratios plus saturating value give automatic balance and auto-pivot away
  from a filled stockpiled good toward uncapped goods — the mid/late-game payoff —
  with one legible control.
- The pure deterministic allocator is replay-safe, test-trivial, and directly reusable
  by the AI (a single solver, not two).
- Off-the-top build-slots plus `enabled` plus no-auto-gate keep construction cost
  visible and supply disruption meaningful, with one player lever per channel.

**Negative:**

- The owned record forecloses fine-grained ECS parallelism *within* a settlement.
  Accepted: within-settlement work is serial anyway; the across-settlement axis is
  what parallelizes.
- Network-scoped saturation couples the allocator to a per-network pre-pass (ADR-0035),
  moving allocation cadence toward per-turn. Affordable (microseconds per settlement);
  flagged.
- Greedy is not optimal under completion bonuses; bounded by keeping bonus magnitude
  modest (the lever that controls both algorithm difficulty and circumvention risk)
  and by the deferred local-improvement seam.
- The derived `claimed`/district indexes must be maintained by every record writer;
  enforced by routing all mutation through the command processor.

## Alternatives considered

- **A Bevy entity per district / per building:** rejected. Sub-settlement structure is
  reached through its owner, not iterated; entities would add archetype churn on every
  state change, a per-instance `NetworkId` shadow (ADR-0024), and relation bookkeeping
  — costs for ergonomics that indexed owned data provides more cheaply, against the
  ADR-0023 discipline.
- **Manual per-slot staffing UI:** rejected as micromanagement (pillar 0.6). The player
  sets intent (priorities); the allocator executes.
- **Absolute per-yield production targets:** rejected for priority ratios — absolute
  numbers are harder to set and create an "above achievable maximum" edge case.
- **Pure slot-greedy without bundles:** rejected. Threshold completion bonuses make
  plain slot-greedy leave buildings one slot short and bank no bonuses; bundles are the
  minimal fix.
- **Automatic build-slot material gate:** rejected — it would smooth over supply
  disruption and undercut the road network's strategic weight (ADR-0030).
- **Building accelerators (PQ, local search) now:** rejected as premature; deferred
  behind the pure interface until profiling/playtest justifies them.

## References

- ADR-0023 (dense tiles; only units and cities are entities — the entity discipline
  this applies to sub-settlement structure).
- ADR-0028 (staffed-work model: the job-slots/build-slots and capacity this allocates).
- ADR-0030 (supply networks; the player-owned-stockpile and no-auto-gate stakes).
- ADR-0034 (job system; build-slots as one labor driver of the construction queue).
- ADR-0035 (transit and field supply; the network-scoped saturation signal).
- `cities.md` §2.6 (growth loop), §2.7 (production model), §2.11 (intent-level
  directives), §2.12 (settlement tiers).
- Design pillars 0.4 (determinism), 0.5 (no over-engineering), 0.6 (intent-level).
