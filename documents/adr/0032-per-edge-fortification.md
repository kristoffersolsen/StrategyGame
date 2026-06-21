# ADR 0032: Per-edge fortification on a shared edge-defense model

**Status:** Proposed
**Date:** 2026-06-20
**Deciders:** Kristoffer

## Context

Cities had no defensive mechanic, which makes early aggression a near-free win and
leaves the settlement layer (`cities.md` §2.12–§2.13) without the defensive
counterweight a 4X needs. The Ancient era needs simple walls. Several things had to
be decided together: where walls live in the data model, how they are built, what
they cost, how they interact with the terrain the map already produces, and how they
feed combat — all while **siege mechanics are deliberately deferred to later eras**,
so Ancient walls ship without a designed counter.

The project already has a per-edge state primitive (roads, `workers.md` §9) and a
per-edge terrain classification (cliffs via `edge_class`, ADR-0012; rivers and
coast). A standalone "+bonus to defender" wall system would sit beside that terrain
rather than composing with it, leaving "use natural blockers" an emergent hope
rather than a mechanic.

## Decision

**Walls are per-edge state, worker-built, on a shared edge-defense model.**

- **Representation.** A wall is **per-edge tile state**, reusing the same edge-state
  machinery as roads (`workers.md` §9) — not a building in a district slot, not a
  tile improvement. Walls are placed on the **edges of district tiles** (the
  built-up perimeter), not on arbitrary claimed-tile edges.
- **Construction.** Worker-built, as a command that sets edge state (`workers.md`
  §3), parallel to roads. Build-slot construction was rejected because build-slots
  produce discrete buildings/units, not edge segments.
- **Cost.** Worker-time **plus a modest base-materials (lumber) charge per
  segment**. This is the deliberate exception to the worker-time-only improvement
  rule (`workers.md` §5): a materials cost makes walling a whole perimeter a real
  guns-vs-walls choice rather than free work for idle workers, and it opens early
  the "construction-grade improvements that cost materials" question that section
  otherwise defers.
- **Shared edge-defense model.** Defensive value attaches to **tile edges**, and a
  single model holds both natural and built blockers in the same units: cliff edges
  (ADR-0012), river edges, coastline, and wall segments all contribute edge-defense.
  When a unit attacks **across an edge**, that edge's defense feeds the defender side
  of the strength ratio (`combat.md` §3.10). "Back the settlement onto a cliff and
  wall only the open sides" therefore falls out structurally.
- **Ancient walls are wooden.** A defender bonus across the walled edge; **no
  damage/destruction model** (a wall is static edge state until its tile is
  captured); **no siege equipment** to counter them. Multiple layers are allowed and
  need no special rule — an attacker pays the edge-defense modifier at each ring,
  which is the "sieges are an investment" property emerging from one mechanism.

**Guardrail invariant (no siege counter yet).** Because walls ship without a siege
counter, the wooden-wall bonus must be tuned so a walled tile is **always takeable
by direct assault** — bloodier and slower, never impregnable. A bonus high enough to
make a walled city unassailable would stall Ancient warfare until a later era. This
is a balance dependency on the unit-strength model (`known-issues.md` §5.4), not a
free parameter.

## Consequences

**Positive:**

- Reuses the existing per-edge state machinery (roads) — no new architecture
  (pillar 0.5).
- Unifying walls with natural blockers makes "use the terrain" structural, not an
  emergent hope, and reads directly off the cliff/river/coast data the map already
  produces (pillar 0.1).
- Edge-based defense slots into the existing strength-ratio computation as one more
  defender term (`combat.md` §3.10).
- Leaves a clean seam for siege: later eras add wall damage/destruction and siege
  equipment that act on wall edge-state, without reworking the model.
- The materials cost makes perimeter walling a genuine economic decision and ties
  defense into the same scarcity that governs the economy.

**Negative:**

- Walls without a siege counter are a balance knife-edge; the guardrail invariant
  above must hold, and it cannot be validated without rough unit-strength numbers
  (`known-issues.md` §5.4).
- Per-edge state for both roads and walls increases edge-state surface area
  (acceptable; shared machinery).
- The materials cost is a deliberate deviation from the worker-time-only rule
  (`workers.md` §5), so that rule now has a named exception to remember.
- Worker-time is now traded off across economy, logistics, **and** defense, which
  raises the stakes on tuning the worker's own create/upkeep cost correctly
  (`workers.md` §5).

## Alternatives considered

- **Standalone wall bonus, separate from terrain:** rejected. It leaves "natural
  blockers reduce wall need" as an emergent hope rather than a mechanic, and
  duplicates a defensive-modifier concept the terrain edges already imply.
- **Walls as a district building (a slot):** rejected. Walls are perimeter edges,
  not tile-interior infrastructure; a per-side model is both more realistic and a
  better fit for the existing edge-state primitive.
- **Walls built via build-slots (consume materials like buildings):** rejected.
  Build-slots produce discrete buildings/units; an edge segment is not one. Routing
  walls through the worker edge-job path is the consistent fit.
- **Worker-time-only walls (no materials):** rejected as the primary model. It makes
  full walling a no-brainer for idle workers and removes the guns-vs-walls tradeoff;
  the materials cost is the point.

## References

- `workers.md` §3 (wall worker job), §5 (the materials-cost exception; worker-cost
  tuning lever), §9 (edge-state command model).
- `combat.md` §3.10 (per-edge defense in combat resolution), §5.4 cross-ref via
  `known-issues.md` (unit-strength validation dependency).
- `ancient-content.md` §6 (walls in the Ancient infrastructure set).
- ADR-0012 (cliffs / `edge_class`), ADR-0030 (roads as per-edge state).
- Design pillars 0.1 (realism backdrop), 0.5 (no over-engineering, reuse).
