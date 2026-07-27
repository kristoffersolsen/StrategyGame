# Workers and Tile Improvement

This document owns the **worker-built tile-improvement layer**: the baseline
extraction that turns claimed land and placed deposits into yields. It is the
connective tissue named by `resources.md` §7 (the deposit-to-yield bridge) and is
deliberately separate from districts, which are the high-efficiency multiplier
layer built through the city production economy (`cities.md` §2.7, §2.13).

This doc is expected to grow: the automation interface, its resolver quality, and
the UI are later work (the AI/QOL phase). What is fixed now is the action *model*
and the ancient-era improvement *set*. The numbering is doc-local; assign a
global chapter number when slotting it beside the existing design docs.

---

## 1. Scope: the baseline layer, and district siting

**Decision (revised, ADR-0030):** Workers build **everything outside a city**. On
claimed tiles they build **tile improvements** — farms, mines, forestry, and the
like; on owned land they **place district footprints** (the act of siting a
district's fixed multi-tile shape, `cities.md` §2.13) **and construct the buildings
inside them**. A **city** builds only its own interior, through the build-slots of
its staffed-work economy (`cities.md` §2.7); an **outpost** — whose center cannot
build (`cities.md` §2.12) — is developed entirely by workers. Worker-built
construction still **consumes network materials**, drawn demand-first at the build
site (`resources.md` §9), so an outpost with no road to a city can have its tiles
improved (worker-time, little or no materials) but cannot have its buildings
constructed. This supersedes the original "workers do not build districts" scope:
workers now site districts **and** construct their buildings; only a city's own
interior is build-slot work.

The improvement and district layers remain distinct, as separated in `cities.md`
§2.13:

- **Improvements** are the low-efficiency baseline on claimed tiles, generally
  outside the district zone.
- **Districts** are the high-efficiency multiplier; a specialization district
  adjacent to worked tiles far outproduces the same land worked without one.

That efficiency asymmetry is a load-bearing invariant (`cities.md` §2.13): it is
what keeps outposts and the core-plus-feeder empire shape (§2.9) meaningful. This
document must not erode it — improvements stay the baseline, never a substitute
for district development. What changed is who builds **outside** a city — workers
now site districts and construct their buildings — not the asymmetry between
improvements and districts, which is untouched.

**Rationale:** Making workers the single way to change anything outside a city
keeps the two-axis model legible (build infrastructure vs. work land) and gives one
interaction model for the whole exterior — no per-outpost build-queue, and no
centralized remote-build-queue stuffing a city's own queue with its outposts' work.
It is **not** a parallel construction path (pillar 0.5): build-slots and
worker-time are two labor inputs to one progress machine (materials invested
against a cost, completion flips build state), and worker-time already builds
improvements and roads, so constructing outpost buildings reuses that channel. The
strategic constraint is preserved by **materials**, not labor: buildings draw
network materials, so an unconnected outpost still cannot build, only improve. The
cost is that worker-time becomes the bottleneck for *all* exterior development,
which makes worker create/upkeep cost the load-bearing tuning lever (Section 5) and
requires worker tasking to be intent-level (§0.6) rather than per-building micro
(ADR-0030).

---

## 2. One mechanism, many named entries

**Decision:** There is a **single improvement mechanism** — a worker occupies a
tile for a build duration, after which the tile gains an improvement that yields
into the appropriate pool (`resources.md` §1–§3, §7). Distinct improvements
(farm, mine, quarry, plantation, pasture, camp, …) are **content-registry
entries** over that one mechanism (stage 3.7 content crate), differing only in
name, valid terrain/resource, and yield — not in mechanics.

**Rationale:** Named granularity is therefore nearly free: adding a "quarry" or
"plantation" costs a registry row, not a system. The realism-flavor desire (a
pasture is not a mine) is satisfied at the content layer without multiplying
mechanics. This also means the recurring "how granular should resources be"
question is not a mechanical-cost tradeoff — the constraint is the player's
decision load and micro burden (Sections 4 and 6), not implementation cost.

**Risks:**
- A single mechanism with many skins can tempt later content into encoding real
  mechanical differences as "just another entry," reintroducing the proliferation
  this avoids.

**Mitigations:**
- Mechanical differences (e.g. an improvement that also exerts zone of control, or
  one that costs materials) are a *new mechanism* and get designed as such, not
  smuggled in as a registry entry.

---

## 3. Ancient-era functional set

**Decision:** The Ancient era uses a small functional set; resource-specific names
are skins over these.

| Improvement | Yields | Valid on | Covers |
|-------------|--------|----------|--------|
| Farm | Food | Arable land (grassland/plains, near water) | wheat; "plantation" skin for crop luxuries (spices) |
| Mine | Materials, strategic deposits | Hills, mountains, deposit tiles | stone ("quarry" skin), copper, iron, gold ore |
| Pasture | Food or horses | Open flat land | horses; grazing |
| Forestry / lumber camp | Lumber (base materials) | Forest tiles | forest tiles; "camp" skin for forest luxuries |
| Fishing | Food | Coast / fish tiles | fish |

This set covers every Ancient-era resource (`resources.md` §8). It is a content
list, expected to grow; the five functional verbs are what the mechanics and the
tech tree (Section 8) actually reason about.

**Roads — worker-built network infrastructure (ADR-0030, ADR-0035).** In addition to
the five yield improvements, the Ancient worker builds **roads**. A road is not a
yield improvement: it is **tile infrastructure** (`infrastructure: Field<u8>`,
ADR-0035) — **not** edge state — and the supply network's connectivity is computed
over road tiles (`cities.md` §2.9, `resources.md` §9). Roads carry the **network
materials** a worker draws on to construct an outpost's buildings (`cities.md`
§2.12). They are the era's first logistics layer; without them an outpost cannot be
built up and distant settling is hard — the intended pressure. A road is **worker
construction**, so, like an outpost building, it **draws network materials** through
the worker's materials buffer (ADR-0035, Section 5) on top of worker-time — which is
what makes road-spam self-limiting through the economy rather than free work for idle
labor. The per-segment magnitude is tuning, deferred.

**Walls — worker-built defensive segments (ADR-0032).** Among the constructs the
worker builds is the **wall**, the Ancient era's settlement defense — on an
**outpost**'s tiles (a city's own walls are build-slot work, by the two-axis rule of
§1). A wall is a one-sided owned **`(tile, facing)` segment** — built on one of the
settlement's own district tiles and facing outward — **not** a tile improvement and
**not** an edge primitive: the far side belongs to no one until an enemy walls it,
and if they do, that is a second, separately owned segment. Segments sit on the
**outward faces of district tiles** (the built-up perimeter). In combat the segment's
bonus is **composed with any natural blocker on the crossed boundary at resolution
time** (`combat.md` §3.10), so backing a settlement onto a cliff and walling only the
open sides still falls out — the cliff already defends those faces. Unlike
improvements, a wall costs **worker-time *and* a modest base-materials (lumber)
charge per segment** (the deliberate exception in Section 5), drawn through the
worker's materials buffer (ADR-0035). Ancient walls are **wooden**: a defender bonus
across the walled face, surmountable by assault, with no damage model yet — siege is
a later era (`combat.md` §3.10, ADR-0032).

**Open questions:**
- Whether pasture splits into a food variant and a horse variant or stays one
  entry parameterized by what the tile offers. Content-layer decision; defer.

---

## 4. Dictated where there is no choice, chosen where there is

**Decision:** On a tile carrying a **resource deposit**, the improvement is
**dictated** by the resource (copper → mine, horses → pasture, spices →
farm/plantation) — there is exactly one sensible extraction, so the player makes
no choice and issues no real decision beyond "develop it." On a **plain tile**
(no special resource) with suitable terrain, the player faces a small genuine
choice among terrain-valid improvements — typically food (farm/pasture) vs.
materials (mine) vs. grazing.

**Rationale:** This puts the player's attention where a decision actually exists
and removes it where it does not. N dictated improvement types are N names for one
non-decision; piling them on adds busywork, not depth, and cuts against the
systems-comprehension pillar. The plain-tile food-vs-materials choice is the
interesting one and is preserved. The larger strategic choices remain at the claim
layer (`cities.md` §2.8) and the district layer (§2.7), where they belong.

---

## 5. Cost: worker-time only (Ancient era)

**Decision:** In the Ancient era an improvement costs **worker-time only** — the
worker is occupied for a build duration — and does **not** consume the building-
materials pool. The production cost was already paid when the worker unit was
built.

**Rationale:** Charging materials per farm is bookkeeping the era does not need,
and the worker is itself the cost gate (you spent production to have one). Keeping
the materials pool for districts, buildings, and units (`cities.md` §2.7) keeps
the early economy legible.

**Walls are the one exception — and a deliberate one (ADR-0032).** Walls (Section 3)
are worker-built but **do** cost base materials per segment, opening early the
"construction-grade improvements that cost materials" question this section
otherwise defers. The reason is design, not bookkeeping: a materials cost is what
makes walling a whole perimeter a real **guns-vs-walls** choice (the lumber spent on
walls is lumber not spent on units or buildings) rather than a no-brainer for idle
workers. Improvements stay worker-time-only; walls are the named exception.

**The worker is the cost gate, which makes the worker's own cost load-bearing.**
Because improvements, roads, and now walls are all gated by worker-time, the
**creation and maintenance cost of the worker unit itself** is one of the most
important tuning levers in the early game — more so than any single improvement's
duration. Set worker create/upkeep cost too low and the entire baseline layer
(economy, logistics, *and* defense) gets done trivially, collapsing the scarcity the
two-axis model and the supply network depend on; set it too high and the baseline
starves and outposts never develop. Walls sharpen this: worker-time is now a single
pool traded off across **economy (improvements), logistics (roads), and defense
(walls)** simultaneously, so worker scarcity is the knob that balances all three
against each other. Workers also draw food upkeep like other units (`resources.md`
§9, ADR-0031), so both the build cost and the ongoing upkeep are levers. This must
be tuned with the whole baseline layer in view, not per-improvement; flagged as a
first-class tuning concern for the economy prototype.

**Open questions:**
- Build duration per improvement, and per wall segment (tuning; no magic number
  committed).
- Worker creation cost and food upkeep — load-bearing per the note above; tune
  against the whole baseline layer, not in isolation.
- The per-segment materials cost of walls (tuning; the structural decision that
  walls cost materials is settled, the magnitude is not).
- Whether multiple workers on one tile shorten the duration (Civ-4-style) or not
  (Civ-5-style). Leaning "no stacking speedup" for legibility; defer.
- Whether replacing an improvement (farm → mine on a plain tile) refunds partial
  progress. Default: no refund, consistent with the §2.12 reconfiguration default.
- Whether *other* later-era construction-grade improvements also cost materials
  (walls have now opened this); revisit per era.

---

## 6. The action layer: manual primary, automation deferred

**Decision:** Tile-improvement actions are issuable two ways, over **one shared
action path**:

- **Manual per-tile orders** — the player directs a worker to build a specific
  improvement on a specific tile. This is the **primary** interface and the
  **first to implement**.
- **Automation directive** — a higher-level instruction (Section 7) that issues
  the same per-tile actions automatically. This is **deferred** to the AI/UI
  phase.

**Manual always overrides automation.** A manual per-tile order takes precedence
over whatever the automation would have chosen for that tile.

**Model invariant (recorded now even though automation is deferred):** the action
path is designed from the start to be drivable both manually and by an automation
directive, sharing one execution path. Implementing manual-only and retrofitting
automation later is rejected — it forces a teardown, the same trap as manual-first
allocation (`cities.md` §2.7) and manual-first loadouts (`combat.md` §3.13). This
is the **third instance** of the §0.6 intent-level pattern; all three reference
the one rule: the player specifies at the level where intent changes, automation
drives the same primitive actions, manual is always the override.

**Why the override is permanent, not a stopgap for a weak resolver:** automation
cannot prioritize correctly *in principle*, because the right improvement
sometimes depends on information the resolver does not have — that the player is
about to go to war and needs materials over food, is racing a wonder, or is
baiting an opponent into contested land. The missing input is the player's
*intent*, which no quality of AI recovers. The manual override is therefore the
channel for information the resolver structurally lacks, and it stays first-class
permanently; later AI work does not obsolete it.

**Rationale:** Manual-first is both the testable core (a deterministic per-tile
action is straightforward to verify) and the substrate automation will later
drive, so prioritizing it is not a detour. Recording the shared-path invariant now
costs one paragraph and prevents a rebuild.

**Risks:**
- "Prioritize manual to test" sliding into "design manual-only," reintroducing the
  retrofit trap.

**Mitigations:**
- The shared-path invariant above is the guard; an automation directive must be
  expressible against the same action API the manual path uses, even before the
  directive itself is built.

---

## 7. Automation granularity (working assumption, deferred)

**Working assumption (not locked):** the automation directive attaches at the
**settlement** level — "this settlement auto-develops its claimed tiles toward its
specialization." Per-tile manual orders within that settlement override the
automation.

**Rationale:** Settlement-level matches the specialization-biased claim growth
already in `cities.md` §2.13 (the settlement has a specialization to develop
*toward*), and it keeps workers as fungible execution rather than entities you
assign policies to. Per-worker directives are micromanagement wearing an
automation costume. Per-empire automation is a later convenience that should
*compose* from per-settlement directives, not replace them.

**Status:** Deferred to the AI/UI phase per the project's QOL-tools priority. The
resolver's quality and the directive UI are explicitly later work. Only the
granularity *assumption* and the override semantics are recorded here so the
manual layer is built against a compatible shape.

---

## 8. Tech-tree interface

**Decision (interface only; the tree is undesigned):** Because the functional set
is small (Section 3), the tech tree gates *capabilities and upgrades*, not a
checklist of improvement types. Examples of the shape, not the content: a tech
enables farms, another enables mines, another enables pasture; later techs
*upgrade* existing improvements (irrigation → better farms) rather than adding
parallel improvement types. Districts (the multiplier layer) are gated separately
through the production/tech systems.

**Rationale:** Same philosophy as the chassis/component model (`combat.md` §3.13):
research capability tiers, not redundant entries. This keeps the Ancient tech tree
from degenerating into a content checklist and lets it inherit a coherent five-verb
set when it is designed.

**Note:** This partially constrains the still-undesigned tech tree (improvement
*enablement* and *upgrades* are tech nodes). Flagged so the tree stays consistent
when built.

---

## 9. Architecture note

An improvement is **tile state** (a dense `Field<T>` value), not an entity —
consistent with the runtime model where entities are reserved for units and
cities. A worker is a unit (entity, `NetworkId`). A worker's build action is a
**command** carrying `PlayerId`, applied by the authoritative core (`sim`), which
sets the tile's improvement state; the manual path and the future automation
directive both emit commands over this one path (Section 6). **Placing a district
footprint, constructing a building outside a city, building a road, and building a
wall segment are likewise worker commands** — each setting authoritative state of its
own shape: an improvement and a road are **tile** state (`infrastructure`, ADR-0035),
a wall is a one-sided owned **`(tile, facing)`** segment (ADR-0032), and a district
footprint is the placed shape over claimed tiles. None of these is an edge primitive;
the engine grows no general edge state (ADR-0035). The one exception is a **city's
own** construction — districts, buildings, and walls on a city's tiles are build-slot
output of the staffed-work economy (`cities.md` §2.7, the two-axis rule of §1), not
worker commands. So the worker layer emits *siting*, *construction* (outside a city),
*road*, and *wall* commands.
Determinism (pillar 0.4) requires the build action and any tie-breaking (e.g.
which improvement an automation directive selects, or nearest-first surplus
routing, `resources.md` §9) to be order-independent and reproducible. Detailed
runtime shaping is deferred to implementation; this note fixes only that
improvements, roads, wall segments, and district footprints are state set via the
command layer, not unit-local mutation.

---

## Cross-references

- `resources.md` §7 — the deposit-to-yield bridge this layer realizes.
- `resources.md` §1–§3, §8 — what each improvement yields; the Ancient resource
  list it must cover.
- `cities.md` §2.13 — districts vs. claimed tiles; the load-bearing efficiency
  asymmetry this layer must not erode.
- `cities.md` §2.7 — districts as the multiplier layer; the §0.6 allocation
  pattern this shares.
- `cities.md` §2.8 — claim vs. develop as separate actions; where the strategic
  choice lives.
- `combat.md` §3.13 — the loadout instance of the same manual-primary pattern.
- §0.6 / `cities.md` §2.11 — the canonical intent-level / anti-tedium pattern;
  this is its third instance.
- `resources.md` §9 — supply network, storage, and the goods roads carry.
- `ancient-content.md` — the Ancient improvement and infrastructure content list.
- ADR-0030 — supply networks; records workers placing district footprints,
  constructing outpost buildings, and building roads.
- ADR-0035 — transit and field supply; roads are tile infrastructure (not edge
  state), and worker construction (roads, outpost buildings, walls) draws network
  materials through the worker's materials buffer.
- ADR-0032 — one-sided owned `(tile, facing)` wall segments; records walls as a
  no-slot build target (worker work on an outpost, build-slot work in a city) and the
  materials-cost exception.
- `combat.md` §3.10 — the composed, per-assault defense model wall segments feed
  into.
