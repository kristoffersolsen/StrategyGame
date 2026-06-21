# Brainstorming document
This document is a place to keep ideas for potential future implementations. Nothing here is committed to the plan,
and will move to design documents when mature enough.

## Resources
There should be several archetypes of resources.

### Basic Resources
Basic resources, like food, are what the player directly accumulates,
and are used to sustain and expand the empire. They include:

- Food: used to sustain the population. For the city to grow there must be enough food, and a surplus
  will boost growth. Not having enough food will result in population going down.
- Building materials: this gathers up stone, lumber, iron, copper, etc. I want this part to be
  more nuanced than other games. It makes little sense that you can place a mine anywhere and get more
  production, so building materials will come from actual resources.
  How this affects production is explained under city management. Each level of resource will
  give more materials. Lumber < Stone < Copper < Iron < Aluminium < Titanium.
- Ammunition: This will come into play in later ages, and be needed to support units.
- Fuel: Also for later eras, needed to support units.

### Luxury Resources
Luxury resources will provide amenities to populations in cities. This can be gold, diamonds, spice, furs, cotton, silk,
etc. They should be varied, and rely on the mineral composition leading to geographical monopolies. This in turn
motivates trade and fighting over resources.

### Strategic Resources
These are linked to the basic Resources, while some of them can be special, like uranium having designated uses. There
should be some reliance on mineral composition, but not to the same degree as for luxuries.

---

## Science
Science is accumulated from buildings and districts in a city. The city center will supply some science, and higher
population will increase science very slightly. This will help early-game, but be negligible mid and late game. Science
buildings will consume some building materials to operate, and later game buildings can rely on fuel, uranium or other
strategic resources.

Science points are used in the tech tree. I am thinking of using the classic linear tech tree, where techs rely on each
other. What I have not decided is visibility and whether the techs should always be placed the same, as these decisions are
linked. If the tech tree does not change there is no point in hiding it. People will simply use wikis and that is bad game design.
If the tech tree does change, having reduced visibility could be interesting. It supports the discover pillar. This is done
in the game Pandora: First Contact. Placement would have to follow some rules, like randomization would only occur within each era.
Additionally some techs depend on each other. You cannot discover battleships before you have oil. Stationary tech tree is simpler, but a
semi-random one is more interesting.

### Technologies

#### Ancient Era

Designed — see `ancient-tech-tree.md` (bronze on the mid-spine as the era-definer,
pinching at Iron Working into Classical). The visibility / semi-random-placement
question discussed above remains open and is recorded there.

---

## Culture
Culture mirrors science in accumulation. Culture should have an effect on how the society develops. This should be in a
tech tree, similar to how civ 6 does it, with a few changes.

---

## Religion



---

## City management

Founding, outposts, districts, and the unified settlement model have been promoted to
`cities.md` §2.12–§2.14. See also the revised §2.4 (settlement founding and
growth) and §2.6 (four dimensions of growth) which now reference the new mechanisms.

Open ideas that remain in brainstorming (not yet mature enough for design doc):

- **Growth-accelerating buildings as a category.** Late-game buildings (cathedral,
  hospital, forum) that drive population growth need a unified category design rather
  than being designed one at a time. Open: rate boost vs. capacity boost, per-empire
  uniqueness, prerequisites, interaction with food and housing systems. Revisit when
  later eras are designed in detail.
- **Geography-influenced specialization.** Whether founding an outpost on ideal terrain
  for its specialization (e.g., mining outpost on a mineral-rich tile) should grant
  automatic bonuses beyond the standard adjacency mechanics. Currently §2.12 has
  specialization as pure player choice; geography-as-influence is an additive idea.
- **Forts and zone of control over claimed tiles.** Brainstorming idea that fort-type
  outposts should claim tiles faster than other specializations and exercise some
  zone of control over surrounding tiles. Deferred until combat ZoC is designed so the
  two ZoC concepts share a single model. Flagged in `cities.md` §2.13.

---

## Eras
Each era has its own focus and challenges. Per the discovery pillar each era builds
upon the previous, introducing new systems.
Not every era has to replace everything or build upon every aspect of the game.
For example Ancient and Classical can share units like archers, and buildings like farms.

Per-era unit, building, and district lists below are scaffolds. They cannot be filled in
meaningfully until upstream decisions resolve: combat balance numbers
(see `known-issues.md` §5.4), specific resource list and rarity tuning (`worldgen.md` §1.16),
and the semi-random tech tree direction under Science above. Premature filling will produce churn.

**The Ancient era is now promoted.** Its content — five district types, the building
set, outpost specializations, the unit roster, improvements, roads, and resources —
is specified in `ancient-content.md` and the system docs (`cities.md` §2.7/§2.13,
`combat.md` §3.13, `workers.md`, `resources.md`). Only the Ancient *tech tree*
remains, deferred until that content picture is committed. The Classical-and-later
scaffolds below stay as brainstorming until each era is designed.

### Ancient Era (Bronze Age)
The ancient era is the introduction to the game. It should be simple,
with few and simple systems for the player to manage. The focus is on establishing your first city,
exploring the surroundings and securing borders.

**Promoted.** The Ancient content is specified outside this document:

- Districts, buildings, outpost specializations, the unit roster, improvements,
  roads, and resources → `ancient-content.md`.
- Systems → `cities.md` (§2.7 production, §2.13 districts), `combat.md` §3.13
  (units/components), `workers.md` (improvements/roads), `resources.md`
  (economy/supply).
- The earlier scaffold here listed "Spearman" as a unit; that is corrected — the
  spear is a bronze weapon component on the warrior chassis, surfacing as the
  named unit "Spearman" (`combat.md` §3.13, ADR-0031).

Open: the **Ancient tech tree** is now designed — see `ancient-tech-tree.md` (a
short hourglass DAG over the content above, bronze on the mid-spine, pinching at Iron Working).

### Classical Era (Iron Age)
focus on science and culture. this should also be an expansion era, where borders are still growing but some contention is happening. inspiration for this is ancient greece.

#### Systems
-culture
-governments
-policies

#### Units
- swordsman (meelee)
- ballista (siege)

#### Districts
- science
- Culture
- production

#### Buildings
- amphitheater (culture) - provides culture
- academy (science) - provides science
- Smith (production) - boosts city production
- walls (defensive) - must be destroyed before moving past

#### Outpost specializations
- Trade Hub

---

### Medieval Era
Also called the dark ages, there should be less focus on science and cultural. borders should be fully formed. happiness and luxury resources should essensial as populations grow.

#### Systems

#### Units
- catapult (siege)

#### Districs

#### Buildings


#### Outpost specialization
- Castle

---

### Renaissance Era

#### Systems


#### Units


#### Buildings

---

### Industrial Era

#### Systems


#### Units


#### Buildings

---

### Modern Era

#### Systems


#### Units


#### Buildings

---

### Future Era

#### Systems


#### Units


#### Buildings
