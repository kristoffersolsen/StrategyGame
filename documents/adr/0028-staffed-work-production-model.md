# ADR 0028: Unified staffed-work production model with district-owned, building-held capacity

**Status:** Proposed
**Date:** 2026-06-20
**Deciders:** Kristoffer

## Context

`cities.md` §2.7 was an open stub ("Not finalized"). The conventional 4X model is
two separate production queues (one for units, one for buildings) fed by an abstract
"production"/"hammers" yield. That model was challenged as both a balancing burden
(a second production currency to tune against the materials economy) and a poor fit
for the project's economy, where building materials are already a tracked resource
(`resources.md` §2). Two alternatives were on the table: priority-based resource
allocation, and Stellaris-style job allocation (buildings create job slots,
population fills them, jobs produce yields).

A prior design session settled the broad shape (staffed work, district-owned
capacity). This ADR records that resolution **and** the refinements made when the
Ancient content was enumerated: that build/job capacity is held by *buildings*
within districts rather than by district types, the co-location rules, and the
single bootstrap slot's placement on the *city* center.

The model must satisfy several standing constraints: the population↔development
coupling that `cities.md` §2.6 depends on (overbuilding should be self-limiting via
staffing), the district-vs-tile efficiency asymmetry that keeps outposts meaningful
(§2.13), the hybrid resource gate (`resources.md` §2–§3), and pillar 0.5 (no second
mechanism doing a job one mechanism already does).

## Decision

Adopt a **unified staffed-work model**. One primitive — a staffed position filled
by population — produces two kinds of output:

- **Job-slots** produce a **continuous yield** while staffed (science, culture,
  food, building materials).
- **Build-slots** **consume building materials over time to produce one discrete
  object** (a unit or a building). A shared per-category queue feeds the
  build-slots; **parallelism equals build-slot count**.

Job-slots produce the materials that build-slots consume: it is one economy, with
no separate abstract production currency.

**Capacity is district-owned, and slots are building-held.** Build-slots and
job-slots belong to the **buildings** constructed inside districts, not to the
district type and not to a city-level production stat. The single hardcoded
exception is the **city center**, which holds exactly one serial **bootstrap
build-slot** (warrior, worker, scout, settler, basic buildings, placing districts,
at baseline rate). An **outpost center has no build-slot and cannot build** (its
construction is performed by a road-connected city, ADR-0030). A settlement with no
production buildings has, beyond that one bootstrap slot, no production capacity.

**Co-location:** a building provides its slots at **base rate in any district**; its
**matching** district **boosts** its output and is the **only** place its
advanced/era successors may be built (the basic building is flexible; advanced ones
are district-locked). The lock, not the boost, is the load-bearing specialization
mechanism.

**Settlement-tier scope (`buildable_in`):** a building may additionally carry an
optional scope restricting it to a set of settlement tiers (e.g. outpost-only,
city-only); absent the field, it builds anywhere. This is a third conditioning axis
on the building model, alongside district-match and basic-vs-advanced. It is a
general, bidirectional content attribute — its first use is the Ancient outpost-only
mill (`ancient-content.md`), and later eras use it for city-only buildings — and it
is a first-class building attribute, not a content-registry quirk. Its legibility
cost (a building that cannot be placed somewhere) is mitigated by **disallow-on-build**
(the settlement refuses the placement; no inert building results) and, where
promotion would strand an outpost-scoped building, **destroy-on-promotion** (the loss
is surfaced as a promotion cost, `cities.md` §2.12).

**Three verbs**, never conflated: *place a district* (worker or city-center
bootstrap slot), *build a unit* (build-slot output, leaves to the field),
*construct a building* (build-slot output, lives in its district and adds its
slots).

**Resource gating is hybrid:** the critical path is buildable from base-tier
materials; higher tiers (copper, then iron) gate strength and efficiency, never
progression (`resources.md` §2–§3; `combat.md` §3.13 for units).

## Consequences

**Positive:**

- One staffed primitive replaces parallel "jobs" and "queues" systems (pillar 0.5).
- The producer/consumer relation (job-slots make materials, build-slots spend them)
  removes the need to invent and balance a second production currency.
- Staffing couples yield to population directly, so overbuilding past staffable
  population is self-limiting — exactly what `cities.md` §2.6 required.
- Building-held capacity makes the district-owned inversion concrete and testable
  and keeps the feeder-outpost empire shape load-bearing (§2.9, §2.13).
- Co-location exposes a real early build-order decision (hybridize a scarce district
  vs. commit) without letting generalist cities dominate, because advanced
  capability is unavailable off-type.

**Negative:**

- The intuitive default — a city-level production stat that districts increment —
  contradicts the building-held model and will be a standing temptation to
  re-collapse. Flagged as an implementation invariant.
- Co-location's boost magnitude is a tuning knob that, set wrong, makes flexibility
  a trap or a dominant strategy. Mitigated by making the *lock* (not the boost)
  carry the specialization pressure.
- The settlement-tier scope adds a third conditioning axis to the building model.
  This is an accepted cost: it is what makes outpost-favoring (and later city-only)
  buildings possible, and it is built and validated on the low-stakes Ancient mill
  as a deliberate testbed. The legibility risk is mitigated by disallow-on-build and
  destroy-on-promotion. Where it co-exists with a specialization-center yield
  multiplier (the farming center), the two effects compound on promotion and form a
  single tuning surface — calibrate them together (`cities.md` §2.12).

## Alternatives considered

- **Two separate queues (units, buildings) fed by abstract production:** rejected.
  It introduces a second production currency to balance against the materials
  economy and does not couple to staffing, leaving `cities.md` §2.6's overbuild
  limiter weak.
- **District-type-owned slots:** rejected in favor of building-held slots. Owning
  capacity at the district-type level cannot express co-location (a unit-build
  building sited in a housing district) and conflates "what the district is for"
  with "what has been built in it."
- **Pure priority-based resource allocation (no discrete slots):** rejected. It
  loses the legible "parallelism = number of build-slots" model and the discrete
  object construction the genre expects.

## References

- `cities.md` §2.7 (production/allocation model), §2.6 (growth loop), §2.12
  (settlements), §2.13 (districts).
- `resources.md` §2–§3 (materials ladder, hybrid gate), §9 (supply networks).
- `combat.md` §3.13 (chassis/component gate).
- ADR-0030 (supply networks; outpost construction via a connected city).
- Design pillars 0.5 (correctness, no over-engineering), 0.6 (intent-level).
