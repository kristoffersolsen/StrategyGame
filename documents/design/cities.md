# Strategy Game — City and Settlement Design

---

## 2. Cities

### 2.1 City as Entity
**Decision:** A city is its own entity with state (owner, population, queues, treasury, etc.) that references the tiles it occupies via `NetworkId`. Tiles hold an optional back-reference to their owning city for fast lookup; the city is the source of truth.

**Rationale:** Modeling cities as components on tiles duplicates state and creates synchronization problems. Entity + references is clean and matches the multi-tile city design.

### 2.2 Tile Ownership Granularity
**Decision (provisional):** At least two ownership tiers per city — **core tiles** (the city proper) and **developed tiles** (have buildings/districts on them). Possibly a third: **influenced tiles** (provide resources but undeveloped).

**Rationale:** Smaller tile scale creates room for nuance that wasn't possible with Civ-sized tiles. Distinct tiers matter for borders, combat, and visual identity.

**Note:** The concrete spatial mechanism for these tiers is now specified in §2.13 (districts vs. claimed tiles) and §2.14 (empire-level ownership). The original three-tier "core / developed / influenced" framing is effectively superseded by "district tile / claimed-and-worked tile / claimed-but-unworked tile."

### 2.3 The Wide-Play Problem
**Decision:** Punish city spam via **economic pressure**, not via abstract science/culture/happiness penalties.

A city must produce more value than it costs in upkeep (money + resources). Cities without sufficient supporting tiles, resource access, or external supply become net negatives.

**Rationale:** Conventional 4X penalties have no fictional grounding — large real-world empires produce more culture and science, not less. Economic pressure is realistic and creates genuine strategic decisions about empire shape.

**Risks:**
- Economic models have feedback loops in both directions: death spirals (bad cities collapse, hard to recover) and snowballs (good cities run away).
- "Develop the minimum, hoard the surplus" can collapse the smaller-tile granularity into effectively-clustered patches.
- Economic state can be opaque to players ("why did my city collapse?").

**Mitigations:**
- No hard floor or ceiling on city outcomes. Large cities have their own intrinsic problems (housing, amenities, resource demand) that act as a natural counterweight to snowballing. The example domain (housing shortage → unrest → production penalty) is the *shape* of the system; actual implementation will be abstracted (status effects triggered by simple ratios), not simulated.
- Development decisions must be interesting beyond raw economic output — adjacency bonuses, specialization, defensive value — so players have reasons to develop tiles beyond the minimum.
- UI is load-bearing: players must see clearly which cities are struggling and why.

**Risk to revisit:** Simulation scope creep. The temptation to model urban dynamics in detail (Victoria 3 territory) must be actively resisted. Status effects gated by simple ratios are sufficient.

### 2.4 Settlement Founding and Growth
**Decision:** Founding is cheap. Growth is expensive.

Founding produces an **outpost** (see §2.12) via a settler unit at low cost. Outposts are purpose-built around a specialization and can sustain themselves on local production. Meaningful growth into a **city** requires constructing a city hall in the outpost's center district, which demands accumulated population and significant materials — and shifts the settlement from net producer to net consumer.

Further growth into larger administrative tiers (regional capital, metropolis) follows the same pattern via successive center building upgrades, each gating a larger district radius (§2.13).

**Rationale:** Inverts the conventional founding cost model. Speculative outpost placement is acceptable because unsupported outposts are visibly small and economically inert. Real investment goes into promoting and growing the settlements that matter.

**Open question (revised):** Unsupported outposts are mechanically handled by §2.12 and §2.13 — they exist at minimum, working only the tiles their specialization needs, and consume almost nothing. The earlier "unfed city" question (die, persist, subsistence) now applies specifically to cities that have promoted past outpost tier and then lost supply. Working assumption: an unfed city loses population back toward the outpost-tier threshold but does not auto-demote; the player chooses whether to demote-and-rebuild or hold and rebuild supply.

**Risks:**

- Cheap founding can recreate the patchwork sprawl problem if unsupported outposts still have meaningful output. Mitigation: §2.3 economic pressure applies uniformly; an unsupported outpost is functionally near-inert.
- "Economic incentives to stimulate growth" is the weakest of the listed growth mechanisms — money is fungible and accumulates, reducing the choice to "do I have spare cash?"

**Mitigations:**

- Unsupported outposts must be functionally inert (or close to it), not just slow.
- Replace or supplement pure-money growth incentives with mechanisms that force real tradeoffs: **migration** (population moves between settlements, zero-sum within empire), **policies/edicts** (empire-level prioritization), **resource-specific stimulation** (food surplus drives growth, not gold).

### 2.5 Capital Mechanic
**Decision (provisional):** The capital is **not** intrinsically special. Its head start comes from starting conditions (initial settler placed, basic infrastructure, starting resources), not from ongoing capital-specific growth bonuses.

**Rationale:** A permanently-special capital locks the best city to the starting position regardless of geography, undermining the goal of meaningful city placement.

**Future aspects:**
- Decide whether "capital" is a mobile designation (e.g., largest/most-connected city gets administrative bonuses) or just descriptive.

### 2.6 The Four Dimensions of Growth
**Decision:** Growth is multidimensional, not a single "size" number. Four
independent axes, each grounded in a specific mechanism:

1. **Population** — driven by food supply and housing-district capacity.
   Population thresholds gate promotion to higher administrative tiers (§2.12),
   and population drives footprint growth (see growth-loop resolution below).
2. **Footprint** — the settlement's claimed-tile area (§2.13, §2.14). Grows with
   population. Cheap and fast relative to development; its extent and direction
   are shaped by geography, neighbors, and specialization.
3. **Development** — districts built on land within the center building's gated
   radius (§2.13). Expensive and slow, driven by production and materials.
   Distinct from footprint: a settlement can claim many tiles without developing
   them.
4. **Specialization** — chosen at outpost founding via the center building
   (§2.12); reinforced by subsequent district choices (farming district near a
   farming outpost, etc.). Can be reconfigured by replacing the center building.

**Growth-loop resolution.** Population is simultaneously an output and an input.
It is produced by food (worked footprint) and housing (development), and it in
turn drives footprint via population-threshold ring expansion (§2.13). The loop
is: food + housing → population → more footprint, bounded by finite good
farmland (terrain), housing requiring deliberate district investment, and the
per-tier claim-radius cap (§2.14). This gives a deliberate split between the two
spatial axes: footprint (cheap) trails population mostly automatically, while
housing and other development (expensive) is the player's deliberate decision.

Development may outrun population only **within the current tier's radius cap**.
Across tiers, population hard-gates development: promotion to a larger radius
requires a population threshold (§2.12), so a settlement cannot build a
city-scale district count on outpost-scale population. Within-tier overbuilding
(filling district slots faster than population justifies) is not hard-blocked and
is not policed by an elapsed-turn timer. It is bounded by maintenance cost
(empty districts are a net drain) and, pending §2.7, by staffing (an unstaffed
district produces nothing). If a decay mechanic proves necessary after §2.7 is
resolved, it attaches to a learnable cause — unmet maintenance or lack of
staffing population — never to elapsed turns, consistent with pillar 0.2.


**Rationale:** A trading hub and a mining town aren't differentiated by "size" —
they differ on each axis independently. The settlement model in §2.12–§2.14
produces these differences from a single mechanism set rather than treating them
as separate stats to balance. Making population the single growth driver reuses
a well-understood loop rather than inventing a parallel one, and the cheap/
expensive split between footprint and development is what makes the two spatial
axes feel distinct in play. Rejecting the elapsed-turn derelict timer keeps the
limiter tied to cause (pillar 0.2) and avoids per-district manual repair tedium
(pillar 0.6); maintenance and staffing already disincentivize overbuilding
without a third penalty.

**Risks:**
- Within-tier overbuild pressure depends on §2.7. If the production model does
  not couple yield to staffing, maintenance alone may be too weak a limiter and
  the question reopens.
- Population-driven auto-claim can surprise the player if the "best next tile"
  heuristic doesn't match intent.

**Mitigations:**
- The §2.7 resolution should be evaluated against this section: the job-slot
  model (buildings create slots, population fills them) makes the population↔
  development relationship fall out for free and is the leading hypothesis.
- Auto-claim's next-tile selection must use a deterministic tiebreak (pillar
  0.4) and surface an "expand toward" directive plus per-tile override (§0.6,
  §2.13).

**Open questions:**
- Specialization-as-geography influence (§2.12 open question) is independent of
  the growth loop and remains open.

### 2.7 Production Model
**Status:** Not finalized.

The conventional "two queues (units, buildings)" model was challenged. The current direction leans away from it toward something closer to **resource allocation** (production capacity split by priority/policy) or Stellaris-style **job allocation** (jobs produce resources, buildings create job slots).

**Gold is empire-wide; bulk goods are not — a deliberate scope asymmetry.** Gold
(currency) accumulates at the empire level because currency is fungible and pools
at the polity level. Building materials and food do not: they are bulk goods,
provisionally city-local (see open question below). Two scope models coexist in
one economy by design. Gold is yield-only with no sink in the Ancient era (a
deliberate stub; `resources.md` §6). Do not infer from "empire-wide gold" that
materials follow — they do not.

- Whether build-slots draw from a single region materials pool or a city-local
  stock. Still open, interacts with regions/infrastructure (§2.9). Resolved for
  gold only: gold is empire-wide (`resources.md` §6); the materials scope is
  independent of that decision.

**Future aspects:**
- Pick a production/allocation model. This is gated on the four-dimensions interactions discussion.

### 2.8 Expansion vs Development
**Decision:** Claiming a tile (expansion) and building on a tile (development) are **separate actions** with separate costs.

**Rationale:** Conflating them (Civ 6) makes optimal play "always claim aggressively because claiming = function." Separating them lets territory be claimed speculatively/defensively while real investment is reserved for development. Also makes claimed-but-undeveloped territory worth fighting over.

The concrete spatial separation is specified in §2.13: expansion claims tiles, development places districts.

### 2.8 Expansion vs Development
**Decision:** Claiming a tile (expansion) and building on a tile (development) are **separate actions** with separate costs.

**Rationale:** Conflating them (Civ 6) makes optimal play "always claim aggressively because claiming = function." Separating them lets territory be claimed speculatively/defensively while real investment is reserved for development. Also makes claimed-but-undeveloped territory worth fighting over.

The concrete spatial separation is specified in §2.13: expansion claims tiles, development places districts.

### 2.9 Regions and Infrastructure
**Decision:**
- Cities connected by sufficient infrastructure (roads, railroads, etc.) form **regions**.
- Within a region, resources flow freely.
- Between regions, flow requires explicit infrastructure (trade routes).
- Infrastructure has **capacity limits** — a road/rail can only carry so much flow. Exceeding capacity gates further growth.
- Players see per-city and per-region statistics including which cities are struggling.

**Open question:** Are regions **emergent from infrastructure topology** (cities sufficiently connected are automatically grouped) or **player-declared with infrastructure requirements** (player draws region boundaries, system validates connections)? Strong recommendation is emergent; not yet locked.

**Rationale:** Regions provide micro-management abstraction without removing depth. Infrastructure as the gating mechanism unifies the player's complexity-management tool (regions) with their strategic depth tool (logistics). Capacity limits force megacities to sprawl outward and collect resources from multiple sources, which is the intended "city footprint plus supply network" vision.

**Risks:**
- Lategame infrastructure upgrades can devolve into mandatory tedious clicking.
- Logistics complexity can explode without proper abstraction layers.
- Megacity-and-vassals could become the dominant strategy, collapsing empire diversity.

**Mitigations:**
- Multiple infrastructure modes (road vs rail vs port) with tradeoffs, not pure linear upgrades.
- Higher-tier infrastructure has higher cost, maintenance, resource requirements, and vulnerability to disruption.
- Distance penalties on resource flow + diminishing returns on city size + strategic vulnerability of megacities + viable medium-city strategies should keep multiple empire shapes competitive.
- Free flow within regions, infrastructure-gated flow between regions, is the primary abstraction layer.

**Future aspects:**
- Lock in emergent vs player-declared regions.
- Design the infrastructure mode set (road, rail, port, etc.) and their tradeoffs.
- Design the resource flow model in detail.

### 2.10 AI City Management / Organic Growth
**Decision:** AI city management and player-facing "automate this city" use the **same system**, exposed to the player as **policies/directives** (e.g., "prioritize defense," "focus economy," "prepare for war") rather than full auto-pilot.

**Rationale:** AI behavior needs building anyway for opponents. Exposing it as policies gives the player meaningful control over delegation while providing a UI surface for debugging AI decisions. References: Old World's ambitions/council demands, Stellaris's sectors.

### 2.11 City Management Tedium (Flagged for Future Design)

**Concern raised:** Managing cities late game is tedious. Players often want to build, or rebuild after combat, multiple of the same city archetype. Manually queueing production in each city scales badly with empire size.

**Direction (to be designed in detail later):** Apply pillar 0.6 (Specify Intent at the Level Where Intent Changes) to city production:

- Designate a set of cities to continuously produce a specified unit type
- Designate a set of cities to follow an empire-level "doctrine" (e.g., "military production," "economic infrastructure," "rebuild after war") that drives their production queues
- Automatic delivery of produced units to the frontlines or designated rally points
- Empire-level standing directives for tile development matching city specialization
- Alert system for "this directive can't be fulfilled" (missing resources, infrastructure, etc.)

Same pattern, same UI metaphors as combat batch commands and standing orders. The systems should feel unified.

**Future aspects:**
- Full design pass to be done alongside the production/allocation model decision (open topic).
- Ensure consistency with AI city management policies (2.10) — these are the same system from different angles.

---

### 2.12 Settlements as Unified Entities

**Decision:** Outposts and cities are the same entity type at different levels of development. The building occupying the settlement's **center district** determines its current role. Founding produces an outpost. Promotion to a city happens by constructing a **city hall** in the center district, replacing the outpost specialization building that was there. There is no separate promotion event or settlement-type flag — the settlement *is* what its center building makes it.

**Founding mechanism.** A settler unit founds a settlement on a chosen tile. On founding, the settlement is a bare center district (visually: a few tents) with no specialization. The player must immediately begin construction of a center building. Specialization options at outpost tier include mining, farming, fort, and trading post; productive types (mining, farming) are cheaper and faster to complete than military types (fort, castle). During construction the settlement is highly vulnerable — this is intentional and aligns with §2.3 (expansion as economic investment, with real risk).

**Promotion to city.** Once an outpost meets a population threshold and has the required materials, it can begin constructing a city hall in the center district. The city hall replaces the outpost specialization building. Completion of the city hall is the promotion event in mechanical terms; the player experiences it as "I built a city hall," not "I promoted my outpost." The same pattern continues for further administrative tiers (see §2.13 on district radius progression).

**Demotion.** Constructing an outpost specialization building over an existing city hall demotes the city back to outpost. Districts outside the outpost's reduced radius become unusable and must be returned to natural state by workers before the surrounding tiles can be re-used. The cost of resetting a district should be roughly equal to the cost of building it — neither encourages flippant restructuring nor punishes legitimate strategic retreat (e.g., abandoning a frontier city before an unwinnable war).

**Center building tiers.** Each successive center building tier requires higher population and material investment than the last, gates a larger district radius (§2.13), and shifts the settlement further from producer to consumer:

- **Outpost specialization** (mining, farming, fort, etc.): district radius 1. Settlement is a net producer.
- **City hall**: district radius 2. Settlement consumes more than it produces but enables city-scale capabilities.
- **Later administrative buildings** (renaissance, industrial, modern): progressively larger district radius, up to the metropolis cap.

The construction-vulnerability pattern repeats at every tier — building a city hall over an existing outpost specializer takes the settlement offline during construction in much the same way.

**Rationale.**

- Collapses two systems (outposts, cities) into one, reusing the building system. Consistent with §0.5 (idiomatic, non-duplicative architecture).
- The center-building rule is teachable in one sentence and produces all downstream behavior (radius, specialization, role) consistently.
- Resolves the apparent conflict between cheap founding (§2.4) and meaningful expansion cost: founding *an outpost* is cheap; growing it into a city is the expensive part. The §2.4 "growth is expensive" principle is preserved and now has a concrete mechanism.
- Economic pressure (§2.3) applies uniformly to all settlements regardless of name. An outpost that cannot sustain itself is a cheap loss; a city that cannot sustain itself is a serious loss.
- Demotion as "build a different center building" reuses existing machinery rather than adding a special operation, and enables real strategic patterns (Sherman-style retreat-and-deny, economic restructuring at peacetime).

**Risks:**

- Construction-time vulnerability creates "kick them while they're building" gameplay. In contested territory this could make outpost founding effectively impossible without military escort, biasing expansion toward safe interior tiles.
- Player confusion: "I built a city hall, where did my mining building go?" If the replacement mechanic isn't surfaced clearly, players may resent losing the production from the specialization.
- The city hall's "consumer not producer" transition could feel punishing rather than progressive without UI that shows the upside (larger radius, new building options) immediately.
- Population gating for promotion has the same shape as classic Civ "size N city required" — risks feeling arbitrary unless the *reason* (the surrounding land must support the city) is visible to the player.

**Mitigations:**

- Military escort and frontier-defense forces become real strategic primitives; this is consistent with §3.4 (positioning and composition matter more than per-unit tactics).
- Center building replacement must show a clear in-UI "trade-up" affordance: "build city hall here — replaces mining outpost, unlocks 2-radius districts, requires population N and materials M."
- Population threshold for city hall should be *visibly* the same thing the surrounding farmland and housing can support. The check is "can this land carry a city," not "have you waited long enough."

**Open questions:**

- **Starting population at founding.** The settlement needs enough population to operate its specialization building from completion; zero starting population produces a "build housing first" sequencing that contradicts the purpose-built framing. Working assumption: a freshly founded outpost starts with 1–3 population (the settlers' worth), enough to staff the initial specialization.
- **Center building's per-tier population contribution.** Whether each center tier adds a fixed amount of population capacity, or whether population is entirely housing-district-driven. Working assumption: the center contributes a small fixed amount; housing districts add the bulk.
- **Reconfiguration cost.** Switching one outpost specialization for another (mine → farm) goes through the same construction process as founding. Whether to refund any portion of the previous building's cost is open; default assumption is no refund.
- **Growth-accelerating buildings as a category.** The brainstorming example of a "cathedral that attracts population growth" should be designed as a category of late-game buildings (rate boost vs. capacity boost, per-empire vs. per-city, prerequisites) rather than a one-off. Deferred until later eras are designed.
- **Hero / commander units at outpost founding.** Whether settler units carry the same hard-death weight as combat heroes (§3.7) and whether losing one mid-construction is a hard setback or merely a delay.

---

### 2.13 Two-Axis Spatial Model

**Decision:** Settlements occupy space along **two separate axes** that should not be conflated: **districts** (built infrastructure on owned land, used for housing and economic specialization) and **claimed tiles** (territorial claim, used for working land — farming, mining, foresting, etc.).

**Districts.** Districts are built within a radius around the settlement's center, gated by the center building tier (§2.12):

- Outpost specialization: 1-tile district radius (6 adjacent slots).
- City hall: 2-tile district radius (up to 18 slots).
- Higher administrative tiers: up to 3 (and possibly more in late eras), with the metropolis as the natural endpoint.

District construction is bounded by the radius but does not have to fill it. Districts include housing, specialized production (limited factories, refineries, trade terminals), and similar built infrastructure. Adjacent districts can interact (adjacency bonuses) but each district occupies one tile.

**Claimed tiles.** Claimed tiles are the settlement's territorial extent. Tile-working (farms, mines, forestry) happens on claimed tiles *outside* the district zone. Farms and mines are **not** districts; they are improvements on claimed land. The maximum claim radius is larger than the maximum district radius, and grows with the settlement (driver and rate are open questions, see below).

**The asymmetry: districts vs. tile-working.** Districts produce significantly more per tile than tile-working does. A farming district (adjacent to claimed farmland) is far more efficient than the same amount of land worked from the settlement center without a district. This asymmetry is deliberate and load-bearing:

- It preserves the distinct role of outposts. If cities could efficiently work tiles directly, outposts would be redundant.
- It pushes empires toward the intended "core city + feeder outpost" shape (§2.9).
- It rewards investment in district construction over passive territorial accumulation.

**District claim priority.** The first district to be built on a tile claims that tile for itself. Once a tile holds a district, no other district can claim it. This is independent of (and stricter than) ordinary tile claiming — the district *is* the use; the tile is locked to that district until the district is removed.

**Inner ring guarantee.** Settlement minimum spacing (§2.14) is set so that the inner 1-tile ring around any settlement's center is always uncontested by neighbors. This guarantees every settlement, however densely packed, can place at least its inner-ring districts. Outer rings may be contested in dense areas; first-construction wins.

**Rationale.**

- Two-axis modeling is more legible than collapsed single-radius models (Civ-style "city radius"). The player has separate intuitions for "what infrastructure have I built" and "what land do I control"; the system should match those intuitions.
- Supports §2.6 axes 2 (footprint = claimed tiles) and 3 (development = districts) with distinct mechanisms rather than overlapping ones.
- Supports §2.8 (expansion vs. development as separate actions) with a concrete spatial separation: expansion claims tiles, development places districts.
- The district-vs-tile efficiency asymmetry creates the right pressure toward the design's preferred empire shape (few large cities + feeder outposts + region infrastructure per §2.9) without artificial penalties.
- The inner-ring guarantee makes minimum spacing meaningful: settlements can compete for outer territory but are never strangled at birth by a neighbor.

**Risks:**

- Two interacting radii (district and claim) increase the conceptual load. Without good UI surfacing, players may not understand why a tile they can see is unavailable to build on.
- Specialization auto-claiming the "best next tile" can produce surprises if the algorithm's notion of "best" doesn't match the player's intent.
- The district/working efficiency asymmetry must hold across all eras and specializations. If a late-era tile-working tech closes the gap, the outpost role collapses.

**Mitigations:**

- UI must distinguish district-claimed tiles, settlement-claimed tiles, and unclaimed tiles unambiguously. Hovering a tile should show why it's available or unavailable.
- Auto-claim is the default; manual override is always available (§0.6). An "expand toward this direction" directive can replace individual tile-by-tile selection for players who want intent-level control without full manual.
- Tile-working efficiency tech should be calibrated to *narrow but not close* the gap. The asymmetry is a foundational invariant, not a tuning knob to be neutralized.

**Open questions:**

- **Claim growth driver.** Population-threshold-triggered ring expansion is the working assumption (every N population unlocks one new claim tile or one new claim ring), with specialization biasing direction (mining outposts grow toward minable terrain). Other options (time, culture, dedicated buildings) are not ruled out but the population approach has the cleanest interaction with §2.6 axis 1 (population) driving axis 2 (footprint).
- **Initial claim on founding.** Working assumption: a freshly founded outpost claims its center tile plus the immediate 1-ring (the area needed for inner-ring districts). This is consistent with realism — founding a settlement asserts immediate hinterland — and avoids the awkward case of an outpost that can't even place its own first district.
- **Auto-claim behavior when no good tile is available.** Whether the outpost stops claiming, claims sub-optimally, or alerts the player. Working assumption: alert on hard failure (no claimable tile within growth range matches specialization), per §3.3 alert pattern.
- **Forts and zone of control on claim.** Brainstorming mentioned forts claiming faster and exercising ZoC over surrounding tiles. This is intentionally deferred until combat ZoC (§3) is designed in detail; claim-ZoC and combat-ZoC should share a single model.
- **Max claim radius per administrative tier.** Working assumption: scales with district radius (e.g., max claim ≈ district radius + 1 ring), so the metropolis tier has the largest worked-tile area as well as the largest district zone.

---

### 2.14 Tile Ownership and Inter-Empire Claims

**Decision:** Tiles are owned at the **empire** level. Settlements are *assigned* tiles for working from the empire's claimed pool, not ownership-holders themselves. First-claim-wins for inter-empire contests: once a tile is claimed by any empire, it remains that empire's unless transferred by war or treaty. No peacetime border drift.

**Minimum settlement spacing.** Settlements must be at least **5 tiles** apart (center-to-center). This guarantees that with maximum district radius (3, achieved at later administrative tiers), the inner 1-ring around each settlement is uncontested while still permitting competition in the outer rings. At maximum radius both settlements would compete for a single contested ring's worth of tiles — meaningful but not crippling.

**Free reassignment within an empire.** Tiles claimed by an empire can be reassigned freely between that empire's settlements at no cost. A tile worked by outpost A can be reassigned to outpost B without penalty, as long as B is in range. This supports common workflows without friction: founding a new outpost near existing ones, switching which settlement works a depleted resource, redistributing tiles after a center building upgrade enlarges district radius.

**Inter-empire transfer.** Tiles claimed by one empire change hands only via:

- **Conquest:** capturing settlements that work the tiles transfers those tiles to the conqueror's empire. (City-capture mechanics in detail are deferred — see §3.12.)
- **Treaty:** explicit voluntary cession in diplomatic agreements. (Diplomacy systems not yet designed.)

No automatic peacetime border shifts. No "cultural pressure" mechanism. If a player wants territory currently owned by another player, war is the answer.

**Implications for city capture.** Because ownership is empire-level, capturing a settlement transfers its worked tiles to the conqueror's empire pool, not specifically to the captured settlement. The conqueror can immediately reassign them to whichever of their own settlements is in range. This simplifies the capture model: territory follows the conquering empire, not the captured shell.

**Rationale.**

- Predictable borders support §0.2 (knowledge as progression — borders do not drift mysteriously) and §3.4 (strategic-level decisions about territory rather than UI nuisance).
- Defers a complex system (border-pressure modeling) that has decades of evidence of frustrating players when implemented (Civ-series cultural pressure).
- Empire-level ownership matches what the player actually cares about ("does my empire control this land") and supports §0.6 (intent at the empire level, execution at the settlement level).
- Free internal reassignment removes a tedium tax on common workflows and supports the unified-entity model — settlements are different views on the same empire's territory, not isolated fiefdoms.
- 5-tile minimum is a tunable number, but the rule it enforces (inner ring uncontested at maximum district radius) is structural.

**Risks:**

- Aggressive players can spam outposts at minimum spacing to lock out neighbors. The economic pressure of §2.3 must hold tightly enough that mass-outposting is self-defeating; if outpost upkeep is trivial, this could become a dominant strategy.
- Free internal reassignment can be exploited if it cancels infrastructure effects — e.g., reassigning a tile mid-supply-route. Reassignment must not break supply-network integrity (§2.9) silently.
- The 5-tile minimum is a global rule; it may feel arbitrary in edge cases (narrow isthmuses, island groups). Whether to relax it on isolated geography is open.

**Mitigations:**

- §2.3 economic pressure must be enforced at outpost-scale, not just city-scale. A mining outpost in barren territory must not produce more than it costs to maintain, even at low absolute numbers.
- Reassignment that would invalidate active infrastructure or supply contracts should either be blocked or trigger a recomputation with visible consequences. Implementation detail; design intent is "no silent breakage."
- Geographic edge cases (islands too small for 5-tile spacing) accept one settlement only — sufficient for game balance, consistent with realism.

**Open questions:**

- **Spacing on different terrain.** Whether minimum spacing should be reduced across impassable terrain (a settlement on each side of a mountain range is geographically reasonable even at 4-tile straight-line distance). Working assumption: use path-distance, not straight-line, for spacing. Defer until pathfinding is implemented.
- **Settlements lost to attrition or abandonment.** If a settlement is destroyed (war, demotion-and-abandonment, starvation), do its claimed tiles immediately become unclaimed, or do they remain owned by the empire pool? Working assumption: empire keeps the tiles unless the *empire* loses them, but tiles unworked by any settlement for some duration may revert. Deferred.
