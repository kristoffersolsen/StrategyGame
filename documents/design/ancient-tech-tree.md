# Ancient-Era Tech Tree

This document owns the **Ancient progression DAG**: what the player starts with,
and the prerequisite graph of technologies that unlock the rest of the Ancient
inventory. It sits on top of `ancient-content.md` (the content it gates) and the
system docs that own each unlocked mechanic. The numbering is doc-local; assign a
global chapter number when slotting it beside the other design docs.

---

## 1. Scope and design philosophy

**Decision:** The Ancient tech tree is a short, wide-then-pinched dependency graph
built on four commitments:

- **Capability bundles, not single unlocks.** Each tech enables a *capability* plus
  the district/building/improvement that realizes it — never a bare content entry,
  never a checklist of one-offs (`workers.md` §8). With roughly twenty inventory
  items and ten techs, each node is a themed bundle.
- **A wide first tier.** The opening techs are independent and force a real choice
  of early direction — production, military, expansion, or (for coastal starts)
  sailing — rather than a single forced order.
- **Bronze defines the era; iron gates the exit.** Bronze Working sits on the
  mid-spine, so the player works toward it, gets it partway through, and wields it
  (the Spearman, the Furnace, the copper materials tier) across the back half of the
  era — bronze *defines* the Ancient era rather than labeling its exit. The hourglass
  **neck is Iron Working**, the era's capstone and transition gate: the branches
  narrow to that one tech, and the iron *discovery* (not bronze) is what points
  forward into the Classical (Iron) era. This keeps the one unavoidable cross-branch
  dependency — military's bronze weapons need production's metallurgy — as a clean
  spine step rather than an awkward arrow, without making the era's signature
  material its parting gift.
- **A short era.** The Ancient era is the introduction and must not overstay. The
  critical path to the Classical transition is four techs deep (Construction →
  Mining → Bronze Working → Iron Working); breadth is optional.

**What this document fixes and what it defers.** It fixes the **dependency graph**
(which tech requires which) and each tech's **unlocks**. It does **not** fix science
costs, per-tier pacing, or whether node *placement and visibility* are randomized
within the era (the Pandora-style semi-random idea under `Brainstorming.md` →
Science). Those are tuning and a separate, still-open decision; if semi-random
placement is later adopted it may warrant its own ADR, since it is a cross-cutting
UI and determinism concern.

---

## 2. Starting state (turn zero)

**Decision:** The player begins with a **settler, a warrior, and a worker**.

- The **settler** founds the capital, which begins at **city** tier — a city center
  with the one bootstrap build-slot (`cities.md` §2.12, ADR-0028).
- The **warrior** defends the otherwise near-defenseless capital (genre standard).
- The **worker** is in hand from the start because the entire baseline layer
  (improvements, roads, walls, district siting) is worker-gated; without a starting
  worker the only sensible opening move is "build a worker," a forced non-choice.
  Starting with one frees the bootstrap slot's first build to be the real decision.

**Buildable at turn zero**, from the city center bootstrap slot: warrior, worker,
**scout** (the era's exploration unit is available even though one is not granted at
start), settler; the **House**; and the placing of **Housing** districts.

**Worker-buildable at turn zero:** the **farm** and **mine** improvements only — the
ungated Bonus floor (`resources.md` §1, §8). All other improvements, and the
placing and building-out of districts, are tech-gated (Section 3); workers gain
those as the unlocking techs arrive (`workers.md` §1).

**Unlocked content at turn zero:** the **Housing** district and the **House**
building. Nothing else — every other district, building, unit, improvement, road,
wall, and weapon component is behind a tech.

**Rationale.** The start gives exactly the three things with no sensible alternative
(settle, defend, develop) and leaves everything directional behind the first tier.
The floor (farm + mine) is ungated because it is the floor — gating it would
soft-lock unlucky or coastal starts, the same reason stone and fish are never
tech-gated (`resources.md` §1). The capital starting as a city is a starting
condition, not a permanent capital bonus (`cities.md` §2.5).

**Open question.** Starting *with* a worker may be reversed after worker
create/upkeep cost is tuned (`workers.md` §5) — that cost is the single lever that
most shapes the early game, so this start choice is settled provisionally and
re-tested then.

---

## 3. The technology graph

**Decision:** Ten technologies in four tiers. Each entry lists its prerequisite,
its unlocks, and the branch it belongs to.

### Tier 1 — Openers (no prerequisite)

- **Construction** *(Production)* — Production district; Workshop (building
  build-slots); Forestry improvement (lumber). The capital-focus opener.
- **Military Tradition** *(Military)* — Military district; Encampment (unit
  build-slots; unlocks the **Archer** chassis). The defense / border-contest opener.
- **Colonization** *(Expansion)* — farming and mining **outpost specialization
  centers** (`cities.md` §2.12); **Roads** (`workers.md` §3, `resources.md` §9). The
  expansion opener — and what makes a *second* settler worthwhile, since outposts
  cannot specialize or connect to the supply network without it.
- **Sailing** *(Geographic — coastal)* — worker **embark**; Fishing improvement;
  a weak **scout boat** (buildable by a settlement that owns a coastal tile). The
  coastal ceiling-unlock: it turns dead water tiles into workable fishing and adds
  naval recon. Off the critical path; surfaced for players with coastal territory so
  it is not an inland dead pick. Embark and the scout boat are the minimal seed of a
  deferred naval layer — no open ocean, no naval combat yet.

### Tier 2 — Branch deepening (one prerequisite)

- **Mining** *(← Construction)* — Mining district (the materials multiplier);
  Silo (storage capacity). (Formerly "Stoneworking"; renamed for clarity, since it
  unlocks the Mining district.)
- **Writing** *(← Construction)* — Library / Lodge (science job-slots). Science is a
  building this era, not a district (`ancient-content.md` §2); this is the deliberate
  investment that accelerates the rest of the tree.
- **Agriculture** *(← Colonization)* — Farming district; Mill (the outpost-only food
  building); Pasture improvement. This is the food-to-periphery system
  (`cities.md` §2.12, ADR-0028).
- **Fortification** *(← Military Tradition)* — Walls (`combat.md` §3.10, ADR-0032);
  Barracks (reduces unit food upkeep).

### Tier 3 — Bronze Working (the era-defining material)

- **Bronze Working** *(← Mining)* — the era's signature advancement, on the
  production spine. Makes **copper** a usable strategic material; unlocks the single
  Ancient **bronze weapon component** (the bronze spear on the warrior chassis
  surfaces as the **Spearman**; `combat.md` §3.13, ADR-0031); unlocks the **Furnace /
  Smeltery** (reduces materials consumed in construction). This is *not* the
  transition gate — the player holds bronze through the back half of the era before
  exiting via iron.

### Tier 4 — Iron Working (the neck / transition)

- **Iron Working** *(← Bronze Working)* — the era's capstone and the hourglass neck.
  **Researching it ends the Ancient era and opens the Classical (Iron) era**, where
  iron becomes the next material tier and the iron sword (the **Swordsman**,
  `combat.md` §3.13) arrives as the first Classical unit. Within the Ancient tree it
  is the threshold; the iron content it points to is Classical-side
  (`resources.md` §2), so the tech opens the door and the next era fills the room.

### Graph at a glance

```
Construction ─────┬── Mining ── Bronze Working ── Iron Working  ⇒  [Classical era]
                  └── Writing
Colonization ───────── Agriculture
Military Tradition ──── Fortification
Sailing (coastal, lateral)
```


The shape is an hourglass: a wide four-opener top, a four-tech deepening tier, then
the production spine carrying through Bronze Working to the single Iron Working neck.
The fan-out above the neck is the Classical era's first tier and is out of scope here.

---

## 4. Bronze the era-definer, Iron the neck

**Decision:** Bronze Working sits on the mid production spine and *defines* the era;
Iron Working is the era's capstone, the hourglass neck, and the Classical transition
gate.

**Rationale.** A keystone is, by construction, the last thing before the transition —
so the era's *defining* material must not sit there, or the player reaches the
Ancient era's signature tech only on the way out of it. Bronze therefore lives on the
spine (Construction → Mining → Bronze Working): the player works toward it, gets it
partway through, and holds it — Spearman, Furnace, the copper materials tier —
through the back half of the era. The neck is **Iron Working**, because iron is the
*next* era's material, so the iron discovery is exactly the right thing to point
forward at the boundary. The one unavoidable cross-branch dependency — bronze weapons
on the always-available warrior chassis are delivered by the production-side
metallurgy line — is then a clean spine step, not a scattered prerequisite.

The critical path is four techs (Construction → Mining → Bronze Working → Iron
Working), still short; everything else is breadth a player picks up by inclination,
not necessity. The progression invariant holds at the neck: Iron Working is research,
not a resource (Section 6).

---

## 5. The first-tier choice

**Decision:** The opener a player researches first sets their early identity:

- **Construction** commits to the capital — parallel construction (Workshop) and the
  production district that hosts it.
- **Military Tradition** commits to arms and borders — the military district, the
  Encampment, and the Archer to contest nearby land.
- **Colonization** commits to expansion — outpost specialization and the roads that
  connect and supply them.
- **Sailing** is the coastal player's fourth option — develop the water rather than
  only the land.

**Rationale.** For an inland start this is a genuine three-way opening tension; for a
coastal start it is four-way, with the map itself widening the decision space. Sailing
is an opt-in ceiling-unlock, not a tax — a coastal city is viable on its land floor
without it (Section 6) — so it competes as a real branch rather than a forced detour.

**Risk.** Four openers for coastal players slightly dilute the production / military /
expansion tension that drives inland choices. **Mitigation:** accepted — it is
geographically-driven richness, and Sailing's payoff (working otherwise-dead water)
is distinct enough not to simply dominate the others.

---

## 6. Progression invariant

**Decision:** No era transition and no critical-path node requires a strategic
resource. **Iron Working** (the transition gate) and **Bronze Working** are both
**researched with science**, so an unlucky start can advance the spine and cross into
the Classical era regardless of deposits: a copper-less start can research Bronze
Working but cannot *build* bronze components, and an iron-less start can research Iron
Working and transition but cannot *build* iron components — weaker, not stuck, with
trade as the path to parity (`resources.md` §3, §8). The materials floor the whole
path rests on is reachable from **land** tiles, so coastal and unlucky starts are
viable; worldgen starting-position adequacy is measured on land (archipelago starts
are an accepted hard edge case, `cities.md` §2.14, `known-issues.md`).

---

## 7. Coverage check

Every Ancient inventory item (`ancient-content.md`) is gated exactly once. Nothing is
orphaned; nothing is double-gated.

| Item | Unlocked by |
|------|-------------|
| Housing district, House | start |
| Production district, Workshop | Construction |
| Forestry improvement | Construction |
| Military district, Encampment, Archer | Military Tradition |
| Outpost specialization centers, Roads | Colonization |
| Worker embark, Fishing improvement, scout boat | Sailing |
| Mining district, Silo | Mining |
| Library / Lodge (science) | Writing |
| Farming district, Mill, Pasture improvement | Agriculture |
| Walls, Barracks | Fortification |
| Copper (usable), bronze weapon component, Spearman, Furnace | Bronze Working |
| Classical era transition (opens the iron tier / Swordsman, Classical-side) | Iron Working |
| Warrior, worker, scout, settler; farm + mine improvements | start |

---

## 8. Open questions

- **Fixed vs. semi-random placement / visibility** within the era (the Pandora-style
  idea, `Brainstorming.md` → Science). The dependency graph here is fixed; whether
  node positions or visibility are randomized per game is undecided and would be a
  cross-cutting UI/determinism decision (possible future ADR).
- **Science costs and per-tier pacing**, and overall era length — tuning, deferred.
- **Bronze for the archer.** Whether the single bronze component also applies to the
  archer chassis (bronze arrowheads) or only the warrior gets the spear this era
  (`combat.md` §3.13). Leaning: the component generalizes per chassis, surfaced as
  named loadouts; magnitude tuned later.
- **Writing's prerequisite.** Currently gated behind Construction; whether science
  should be reachable off a non-production opener is open.
- **Iron Working boundary.** Whether the iron weapon component (Swordsman) unlocks at
  the instant Iron Working completes (i.e. the last Ancient action) or as the first
  Classical-tree pick. Either reads as "the bridge"; pin it when the Classical tree is
  designed (`combat.md` §3.13, `resources.md` §2).
- **Sailing surfacing rule.** The exact condition under which Sailing is offered
  (coastal territory owned) — pin during UI/visibility design.
- **Starting worker.** Revisit after worker create/upkeep cost is tuned
  (`workers.md` §5).

---

## Cross-references

- `ancient-content.md` — the inventory this tree gates (districts, buildings, units,
  improvements, infrastructure, resources).
- `cities.md` §2.7 (production / build-slots), §2.12 (settlements, outpost centers,
  capital-as-city), §2.5 (capital not intrinsically special).
- `combat.md` §3.13 (chassis/components; bronze spear → Spearman, iron sword →
  Swordsman), §3.10 (walls).
- `workers.md` §1, §3, §5 (improvements, roads, walls; the worker-cost lever), §8
  (gate capabilities, not a checklist).
- `resources.md` §1–§3, §8 (the floor, the materials ladder, the strategic gate, the
  progression invariant; copper Bronze-era, iron Classical-era), §9 (supply network).
- ADR-0028 (production model), ADR-0031 (unit model / bronze component), ADR-0032
  (walls). ADR-0017 (resource placement; copper baseline).
