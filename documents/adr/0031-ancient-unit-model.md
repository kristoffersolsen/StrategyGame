# ADR 0031: Ancient unit model — weapon components over named chassis, food upkeep

**Status:** Proposed
**Date:** 2026-06-20
**Deciders:** Kristoffer

## Context

`combat.md` §3.13 settled the chassis/component model: a unit is a base chassis plus
resource-gated components, and "named ladders" (Warrior → Swordsman → Legion) are
replaced by component lines researched once. But the doc's Ancient-era scope line
listed the base chassis as "warrior, **spearman**, archer" — which contradicts the
model it had just stated, since a spearman is a warrior carrying a spear, not a
distinct chassis. Enumerating the Ancient content surfaced this slip and two
adjacent decisions that were undocumented: where the archer becomes available, and
what units cost to maintain in the Ancient era.

## Decision

**The Ancient base chassis are warrior (melee) and archer (ranged)** — not "warrior,
spearman, archer" — plus the non-combat worker, scout, and settler.

**The spear is a weapon component, not a chassis.** The era's single component slot
is the **weapon**; its augment tier is **bronze** (copper-gated, `resources.md` §3).
A warrior equipped with the bronze spear surfaces as the **named unit "Spearman."**
The same warrior chassis with the Classical iron sword surfaces as **"Swordsman."**
Named loadouts recover the milestone reward of a named unit while keeping the
orthogonal-component structure.

**Archers gate behind a building, not the center.** The warrior, worker, scout, and
settler come from the city center's bootstrap build-slot (ADR-0028); the **archer
chassis is unlocked by the encampment** (a military-district building,
`ancient-content.md`), giving the military district an early reason to exist.

**Ancient unit upkeep is food**, drawn continuously and demand-first from the unit's
supply network (`resources.md` §9, ADR-0030), tying army size to the agricultural
base. The **barracks** building reduces it. This is the economic draw only;
field-supply states (in-supply / foraging / out-of-supply) remain owned by
`combat.md` §3.11.

## Consequences

**Positive:**

- Removes an internal contradiction in `combat.md` §3.13 (a "spearman chassis" in a
  model that says spearman is a loadout).
- Named loadouts (Spearman, Swordsman) keep the visceral named-unit milestone while
  the underlying structure stays orthogonal components on a fixed chassis.
- Gating the archer behind the encampment gives the Ancient military district a
  concrete early purpose rather than leaving all units buildable at the center.
- Food upkeep ties army size to the food economy — a realistic, legible limiter that
  needs no gold sink (still deferred, `resources.md` §6).

**Negative:**

- "Spearman" and "Swordsman" as *named loadouts* (not chassis) must be surfaced in
  UI as units, or players will perceive a loadout system instead of a roster. The
  naming mitigation depends on UI work.
- Food upkeep adds a per-unit economic draw the early game must be tuned around
  (upkeep-per-unit magnitude deferred).

## Alternatives considered

- **Keep "spearman" as a distinct chassis:** rejected — it contradicts the §3.13
  model and reintroduces the named-ladder redundancy the chassis/component model
  exists to remove.
- **All units buildable at the center (no building gate for archers):** rejected —
  it leaves the Ancient military district with no early reason to exist and weakens
  the production-capacity-is-district-owned story (ADR-0028).
- **Gold-based unit upkeep in the Ancient era:** rejected — gold has no sink yet
  (`resources.md` §6); food upkeep is both available and the better thematic and
  strategic fit for the era.

## References

- `combat.md` §3.13 (chassis/components), §3.11 (field supply).
- `resources.md` §9 (food upkeep draw), §3 (copper/bronze gate), §6 (no gold sink).
- `ancient-content.md` (the encampment and barracks buildings; the unit roster).
- ADR-0028 (city-center bootstrap slot; district-owned capacity).
- ADR-0030 (the network the upkeep is drawn from).
