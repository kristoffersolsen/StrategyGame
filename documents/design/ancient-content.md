# Ancient-Era Content

This document is the **content inventory** for the Ancient era: the concrete
districts, buildings, outpost specializations, units, improvements, infrastructure,
and resources the era ships with. It is the enumeration layer over the system docs,
not a re-statement of their mechanics — each entry points to the doc that owns its
*why*:

- District and slot mechanics — `cities.md` §2.7 (production), §2.13 (spatial).
- Unit composition — `combat.md` §3.13.
- Improvements and roads — `workers.md`.
- Economy, supply, and resources — `resources.md`.

The inventory exists so the **Ancient tech tree** (`ancient-tech-tree.md`) has a
complete, fixed list of things to gate: every tech node enables or upgrades
something on this list, nothing more (`workers.md` §8 — capabilities, not a
content checklist). The numbering is doc-local.

---

## 1. Districts

The Ancient era ships **five** district types. Districts are fixed multi-tile
footprints holding buildings; the buildings hold the job-slots and build-slots
(`cities.md` §2.7, §2.13, ADR-0028, ADR-0029). The economic, defensive, and
science *districts* are deliberately **not** Ancient — economic gold has no sink
yet (`resources.md` §6), defensive walls are Classical, and science is a building
this era (Section 2), not a district until the Classical campus.

| District | Footprint (Ancient) | Building slots | Role |
|----------|--------------------|----------------|------|
| Housing | 1 tile | 2 | Population capacity; the jigsaw filler. In-place tier modernization, every era. |
| Production | 2 tiles | 3 | Hosts building build-slots and consumption-reduction. Grows to 3 tiles (Industrial), 4 (Modern). |
| Military | 2 tiles | 3 | Hosts unit build-slots and upkeep-reduction; unlocks the archer. Grows to 3 tiles (Medieval), 4 (Modern). |
| Farming | 2 tiles | 3 | Extraction multiplier for adjacent farmed tiles (the §2.13 asymmetry, made buildable); hosts the mill. |
| Mining | 2 tiles | 3 | Extraction multiplier for adjacent mined tiles. |

**Farming and mining districts are confirmed Ancient content** — they are the core
resource providers, and deferring them would overload the Classical era. Their
mechanism is the `cities.md` §2.13 multiplier: a farming/mining district raises the
yield of adjacent worked tiles far above the baseline worker improvement.

**Sizing — 2 tiles, 3 slots.** Chosen for taxonomy and effectiveness, not micro:
it keeps the clean Ancient split where **1-tile means housing (the filler)** and
**2-tile means working district** (production, military, farming, mining), so
"1-tile" stays a unique signal and the size-as-era-stamp story (ADR-0029) holds; and
a 2-tile footprint has more perimeter adjacency to the worked tiles it multiplies.
Space is not tight: a 1-radius outpost (the 6-tile inner ring) running, say, 2
farming districts plus 2 housing fills the ring and still carries 6 + 4 = 10 building
slots for mills, silos, and houses. Outpost micro is handled by the §0.6
auto-develop directive (`workers.md` §7), not by district size.

**District vs. outpost specialization (clarification).** "Farming" and "mining"
name both an outpost *specialization* (the center-building focus, `cities.md`
§2.12) and a *district* (the buildable multiplier). They are different layers: the
specialization is what an outpost's center is optimized for; the district is built
infrastructure that multiplies adjacent extraction. A farming outpost is good at
working farmland from its center; a farming district built nearby multiplies it.

**Pushing food to the periphery — two stacked levers (`cities.md` §2.12).** A city
hall's radius-2 zone (≈18 district tiles) would otherwise make a giant farming
*city* the space-efficient way to grow food, collapsing the feeder-outpost shape
§2.9 exists to produce. Two explicit levers counter this and both are deliberately
built in the Ancient era as a testbed:

- The **farming specialization center** applies a **food multiplier** over the
  whole settlement's food output.
- The **mill** is an **outpost-only** food building (Section 2).

Both are lost on promotion to a city (the center is replaced; the mills are
destroyed — `cities.md` §2.12), so the loss compounds: promoting a developed farm
outpost surrenders the multiplier *and* its mills. The most-developed breadbaskets
are therefore the least worth upgrading — intended. The city upgrade path is meant
to come from **trade and defense**, not from a settlement's farm output, and the
best farm towns are flatland sites poor in strategics and luxuries. The two levers
compound, so they are a single tuning surface, not two independent ones; calibrate
the multiplier and the mill yield together, and keep the advantage a gradient (a
city must still partially self-feed, or one road-cut starves it — `resources.md`
§9).

---

## 2. Buildings

Buildings hold slots and live in districts. A building provides its slots at base
rate in **any** district; its **matching** district boosts it, and advanced/era
successors are **locked** to the matching district (`cities.md` §2.7, ADR-0028).
The Ancient set:

| Building | Home district | Slot kind / effect | Notes |
|----------|---------------|--------------------|-------|
| House | Housing | Population capacity (passive) | Two houses fill a housing district; the second slot can co-locate an off-type building. |
| Workshop | Production | **Building build-slots** | Parallel building construction beyond the center's bootstrap slot; parallelism = slot count. |
| Furnace / Smeltery | Production | Reduces materials (incl. strategic) consumed by construction | Consumption-reduction, not a production boost (a boost snowballs; reduction has a floor). Stacking is capped. |
| Encampment | Military | **Unit build-slots**; unlocks the **archer** chassis | The military district's early reason to exist; archers gate here, not at the center (`combat.md` §3.13). |
| Barracks | Military | Reduces unit **food** upkeep | Military mirror of the furnace; presupposes the food-upkeep drain (`resources.md` §9). |
| Mill | Farming | **Food yield**; **outpost-only** | Pushes placeable food to the periphery (see §1). Built only in outposts (`buildable_in` scope, below); **destroyed on promotion** to a city. First instance of the settlement-tier scope archetype. |
| Library / Lodge | Any | **Science job-slots** | Science is a building this era, unboosted (no science district yet). The Classical campus boosts it and hosts the academy. |
| Silo | Any | Adds **storage capacity** (materials + food) | Purely logistical — holds, does not produce or build. Buffers a frontier settlement against road severance (`resources.md` §9). |

**Settlement-tier building scope (`buildable_in`).** Co-location (ADR-0028)
conditions a building on two axes — which district, and basic-vs-advanced. The mill
introduces a **third axis: settlement tier.** A building carries an optional
`buildable_in` scope (a set of settlement tiers); absent the field, it builds
anywhere. The mill is `buildable_in: {outpost}`. The archetype is **general and
bidirectional** — later eras get **city-only** buildings the same way (a metropolis
stock exchange) — so it is built once as a scope attribute, not as a one-off mill
hack. This third dimension is a deliberate, recorded extension of the building model
(ADR-0028): its legibility cost (a building that can't be placed somewhere) is
mitigated by **disallow-on-build** (a city simply refuses the mill — no dead
building) and by **destroy-on-promotion** (the transition is a surfaced cost, not a
silent stranding). It must be a first-class building attribute, not smuggled in as a
content-registry quirk (`workers.md` §2).

**Mining district roster (Ancient).** Mining has **no dedicated matching building**
this era — by design. The food levers exist to steer *placeable* farms to the
periphery; copper and the other mined resources are **deposit-dictated**
(`workers.md` §4), so mines go where the ore is and there is little placement to
steer. The mining district earns its place purely through the §2.13 adjacency
multiplier on adjacent mines, and its three Ancient slots are filled by co-located
**silo** and **house**. Its outpost-favoring building arrives next era, using this
same `buildable_in` scope.

The **center** is not a building in this list: it is the settlement center
(`cities.md` §2.12), and a *city* center carries the single bootstrap build-slot
(warrior, worker, scout, settler, basic buildings, placing districts). An outpost
center cannot build.

---

## 3. Outpost specializations

Chosen at outpost founding via the center building (`cities.md` §2.12). Four in the
Ancient era:

- **Farming** — productive; cheap and fast to complete. Its center applies a
  **food multiplier** over the whole settlement's food output (§1, `cities.md`
  §2.12); lost on promotion to a city.
- **Mining** — productive; cheap and fast.
- **Fort** — military; slower and costlier. (Fort zone-of-control over claimed
  tiles is deferred until combat ZoC is designed, so the two ZoC concepts share one
  model — `cities.md` §2.13.)
- **Trading post** — positions for future trade; gold yield is inert in the Ancient
  era (no sink, `resources.md` §6), so its Ancient value is positional, not
  economic.

---

## 4. Units

A unit is a chassis plus components (`combat.md` §3.13, ADR-0031). The Ancient
roster:

- **Warrior** — melee chassis. Built from the city center's bootstrap slot.
- **Archer** — ranged chassis. Gated by the **encampment** building (Section 2).
- **Worker** — non-combat; builds improvements, places district footprints, builds
  roads and walls (`workers.md`).
- **Scout** — non-combat; exploration.
- **Settler** — non-combat; founds settlements.

**Weapon component:** the **spear** — the Ancient bronze melee weapon, copper-gated
(`resources.md` §3). A warrior with a bronze spear surfaces as the named unit
**"Spearman."** The same chassis with the Classical iron sword surfaces as a
**"Swordsman."** There is no separate "spearman" chassis; the era's single component
slot is the weapon.

**Upkeep:** units draw continuous **food** upkeep from their network
(`resources.md` §9), reducible by the barracks. The Ancient set has **no mounted
unit** (horses gate a Classical mounted archetype); add a mounted scout only if one
is later wanted (`resources.md` §8).

---

## 5. Improvements (worker-built)

The baseline extraction layer on claimed tiles (`workers.md` §3). Five functional
verbs, with resource-specific names as skins:

- **Farm** — food (wheat; "plantation" skin for crop luxuries).
- **Mine** — materials and strategic deposits (stone "quarry" skin; copper, iron,
  gold ore).
- **Pasture** — food or horses.
- **Forestry / lumber camp** — base materials from forest.
- **Fishing** — food from coast/fish tiles.

Improvements cost worker-time only in the Ancient era (`workers.md` §5).

---

## 6. Infrastructure

Both are **per-edge state** (not tile improvements), worker-built, reusing one
edge-state mechanism (`workers.md` §3, §9):

- **Roads** — network edges (`resources.md` §9, ADR-0030). The supply network over
  which materials and food are spent, and the means by which a city's build capacity
  reaches an outpost to construct its buildings. The Ancient era's first logistics
  layer. Cost: worker-time.
- **Walls** — defensive edges (`combat.md` §3.10, ADR-0032). Placed on the edges of
  district tiles; a walled edge feeds the **shared per-edge defense model** along
  with natural blockers (cliffs, rivers, coast), so a settlement backed onto terrain
  walls only its open sides. Ancient walls are **wooden**: a defender bonus across
  the walled edge, surmountable by assault, with no damage model (siege is a later
  era). Cost: **worker-time plus base materials (lumber)** — the deliberate exception
  to the worker-time-only rule (`workers.md` §5), making perimeter walling a
  guns-vs-walls choice. Multiple layers are allowed and need no special rule (the
  edge bonus is simply paid at each ring).

---

## 7. Resources

The Ancient economic picture (authoritative list: `resources.md` §8):

- **Strategic — Copper** *(new content)*: the Bronze-Age higher material tier;
  gates bronze components.
- **Strategic — Iron, Horses** *(present, economically Classical)*.
- **Bonus — Stone, Wheat, Fish** *(the floor)*; Lumber as a forest feature.
- **Luxury — Gold ore, Gems, Spices, Wine, Tin** *(gold ore renamed from "gold" to
  resolve the currency collision; tin is new content)*. Each gives one amenity plus
  one secondary yield.
- **Currency / yields — Gold, science, culture**: empire-wide. Gold is yield-only
  (no sink) in the Ancient era.

---

## 8. The tech tree

The **Ancient tech tree** — the progression DAG that gates this inventory — is now
specified in its own document, `ancient-tech-tree.md`: the turn-zero starting state,
four wide first-tier openers (production, military, expansion, and the coastal
Sailing branch), a deepening tier, **Bronze Working** on the mid production spine as
the era-defining material, and **Iron Working** as the hourglass neck that gates the
Classical transition. Every item in this inventory is gated exactly once there (see
its coverage check). The earlier farming/mining-district gap is closed (Sections
1–2), so those districts appear as tree nodes (Agriculture, and Mining — the renamed
Stoneworking).

---

## Cross-references

- `ancient-content.md` — this document.
- `ancient-tech-tree.md` — the progression DAG that gates this inventory.
- `cities.md` §2.7 — staffed-work production, building-held slots, co-location,
  settlement-tier scope.
- `cities.md` §2.13 — district footprints, era-size ladder, antique conversion.
- `cities.md` §2.12 — settlements, outpost specializations, construction model,
  promotion trade.
- `combat.md` §3.13 — chassis/components; the spear and named loadouts.
- `combat.md` §3.10 — the shared per-edge defense model walls feed into.
- `workers.md` — improvements, roads, walls, district siting; the worker-cost
  tuning lever.
- `resources.md` §8 — the authoritative Ancient resource list; §9 — supply.
- ADR-0028 (production model), ADR-0029 (district footprints), ADR-0030 (supply
  networks), ADR-0031 (Ancient unit model), ADR-0032 (per-edge fortification).
