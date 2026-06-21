# Strategy Game — Combat Design

---

## 3. Combat

### 3.1 Tactical Model

**Decision:** Turn-based hex combat.

**Rationale:** Fits everything else designed (hex movement, terrain interactions, supply networks as targets). Genre's natural strength. Real-time-tactical hybrid attempts (e.g., early Total War experiments) historically sacrifice strategic depth for tactical chaos.

### 3.2 Unit Interface: Batch Commands, Not Squads-as-Game-Objects

**Decision:** Multi-unit selection with batch command execution. Units remain individual game objects; "squads" are an interface concept, not a separate game entity.

**Concern this addresses:** Late-game combat tedium of cycling through dozens of similar units (e.g., artillery batteries around a city) each turn, issuing nearly-identical orders one at a time.

**Rationale (alternatives considered and rejected):**
- *Squads as mandatory groupings the player creates and maintains* → adds management overhead; if the squad layer isn't strictly more efficient than the unit layer, players end up managing both. Failure mode of Endless Legend's army stacks.
- *Squads as new game objects with their own mechanics (formations, shared morale, etc.)* → significant design weight; bakes in management overhead before knowing whether mechanics earn it. Considered for v2+ if combat depth needs it.
- *Interface-layer batch commands only* → strictly additive, solves the tedium problem directly, doesn't preclude future depth.

**Selection mechanisms:**
- Drag-box select
- Click + shift for individual additions
- "Select all of type X within range/region" hotkeys
- Saved selections (e.g., "Army Group A" — UI grouping, not game entity)

**Batch command semantics (each action needs defined group behavior):**
- **Fire at target:** all selected units fire if in range. Out-of-range behavior: hold (decision: substitute target vs. hold deferred to playtest).
- **Move to tile:** units pathfind individually to a cluster of tiles around target. Assignment of specific units to specific tiles needs rules (closest-assignment vs. type-based ordering — front-line types forward, ranged behind).
- **Fortify / hold:** trivial, applies uniformly.
- **Skip turn:** trivial, applies uniformly.
- **Attack into adjacent:** all units adjacent to target attack. Resolution order (initiative? melee first?) needs design.
- **Retreat:** complex — direction defined by "away from selected enemy" or "to selected friendly tile."

Actions that don't make sense at group level are grayed out for that selection.

**Risks:**
- Group "move to tile" with many units can produce unintuitive results (units scattering, blocking each other) without careful assignment logic.
- Selections must be robust to unit deaths mid-combat (selection auto-updates).
- Players may want different group semantics than designed (substitute-target vs. hold preference, etc.). Plan for tuning.

### 3.3 Standing Orders

**Decision:** Units can be issued standing orders that persist across turns until cancelled or unfulfillable.

**Rationale:** Pairs naturally with batch commands. Reduces per-turn micromanagement of routine behavior. Implements pillar 0.6 (specify intent at the level where intent changes).

**Example standing orders:**
- "This artillery battery: fire at nearest enemy city each turn"
- "This army: defend this region; engage any enemy that enters"
- "These workers: continue improving this region"

**Mechanism:**
- Orders persist on the unit
- Automatic re-evaluation each turn
- Alert player when order becomes unfulfillable (out of ammo, target gone, enemy retreats, supply runs low)
- Player intervenes when conditions change, not every turn

**Risks:**
- Standing orders that subtly misbehave (e.g., artillery firing at a sub-optimal target while a better one exists) erode player trust in the system.
- Alert fatigue if standing orders flag too often.

**Mitigations:**
- Tune alert sensitivity. Default to alerting only on hard failures (can't execute), not soft sub-optimalities.
- Make order behavior discoverable — player can inspect what a unit is currently planning to do.

### 3.4 Strategic-Level Decision-Making

**Decision (load-bearing principle):** Combat is designed so that **force composition, positioning, and timing** matter more than per-unit micromanagement. The interesting decisions happen above the unit level.

**Rationale:** Once batch commands and standing orders make per-unit management mostly automated, the question "what is the player doing each turn in late-game combat?" needs an honest answer. The answer should be: strategic decisions (where to commit reserves, which fronts to prioritize, when to negotiate, what to build and where to deploy), not tactical micromanagement.

**Design implications:**
- **Unit composition matters more than unit count.** A balanced force of infantry, ranged, and support should beat a homogeneous stack. Forces strategic decisions at army assembly, not just battle execution.
- **Positioning matters more than action sequencing.** Where an army is positioned at the start of a war shapes outcomes more than perfect tactical play in each battle.
- **Strategic information matters.** Scouting, intelligence, and knowledge of enemy supply/movement are first-class systems, not afterthoughts.

**Implications for AI:**
- AI opponents must also think at force level, not per-unit. Otherwise the player has a UI advantage (batch commands) that's not strategic — they're just clicking faster.
- AI design simplified somewhat: force-level reasoning ("what should this army do") closer to human strategic thinking than per-unit reasoning.
- AI issues equivalent batch orders to its forces.

### 3.5 Unit Scale and Representation

**Decision:** Company-scale units (~100-200 soldiers each), one unit per tile (1UPT) with exceptions for rail transit (see 3.8).

**Rationale:** Company scale gives noticeably more units than Civ-style battalion/army representation (visible payoff of smaller-tile design) without crossing into platoon-scale unplayability. 1UPT preserves the tactical positioning depth that justifies the smaller-tile design — stacks would obviate positional play, armies-as-aggregates would hide the granular combat the design enables.

**Implications:**
- Two friendly units can't share a tile (except via rail stacking, see 3.8)
- Movement *through* friendly tiles allowed; ending a turn requires an empty tile
- Wars involve dozens of units per side
- Per-unit decisions become less interesting; force-level decisions become more interesting (reinforces pillar 0.6)

**Risks:**
- Traffic jams when reinforcements can't path past friendly units. Less severe than Civ 5/6 due to higher tile count, but doesn't disappear.

**Mitigations:**
- Auto-pathfinding routes around friendly units when possible
- Explicit "swap positions" unit action for unblocking
- Rail stacking (3.8) provides a fast-transit option for non-combat movement

### 3.6 Unit Archetypes (Soft Rock-Paper-Scissors)

**Decision:** Five archetypes with situational effectiveness modifiers. Every unit can attack every other; composition and positioning determine outcomes, not hard counters.

**Starter archetypes:**
- **Melee infantry:** the line. Strong defense, average offense, cheap, available early.
- **Ranged infantry:** archers, later musketeers. Attack at range, weak in close combat.
- **Cavalry:** fast movement, strong offense against soft targets, vulnerable to formations.
- **Siege:** very strong against fortifications/cities, slow, fragile in open combat.
- **Support:** logistics, engineers, medics. Non-combat or weak-combat with buff effects on nearby forces.

Each tech era refreshes the archetypes with stronger versions (musketeers replace archers, tanks replace cavalry) without changing underlying roles.

**Rationale:** Soft RPS produces composition pressure (you can't win with one unit type) without checklist-play (no unit is helpless against another). Roman-legionary pattern: effective generalists with situational matchup differences.

**Terrain and combat interaction:** all archetypes affected by terrain modifiers — cavalry strong in plains, weak in forest; ranged gains height advantage; siege requires accessible terrain. Geography (height, biome, rivers, canyons, infrastructure) provides the situational layer that makes combat interesting.

**Discovery-pillar compliance (0.2):**
- Tooltips show *effects* ("effective against cavalry") not *formulas* ("+30% strength vs cavalry")
- Combat predictions UI shows verdict ("strong advantage", "even fight", "likely defeat") not breakdown
- Players learn nuances (terrain modifiers, flanking, height) through play

### 3.7 Veterancy and Hero Units

**Decision:** Tiered veterancy with branching specialization at hero threshold. Hard death — no resurrection or wounded-retreat mechanic.

**Tier structure:**
1. **Regular veterancy levels** (low to mid): incremental boosts to attack, defense, movement. Linear progression, predictable.
2. **Hero threshold (high level):** branching specialization — player chooses one archetype:
   - *Shock trooper:* high attack, low defense, AoE damage on attack
   - *Defender:* high defense, AoE defense buff to adjacent friendlies
   - *Commander:* modest combat, strong AoE buffs to nearby units' attack/defense
   - *Scout:* high movement, vision bonus, recon abilities
3. **Support hero variants** (for engineer/logistics/medic units):
   - *Master engineer:* faster infrastructure construction; can build one tier above current tech
   - *Logistician:* increases supply throughput on adjacent infrastructure; reduces nearby unit supply consumption
   - *Field medic:* heals adjacent units each turn; prevents attrition losses

**Hero death: hard.** Heroes who die in combat are gone permanently.

**Rationale for hard death:**
- **Prevents accumulation.** Wounded-retreat or resurrection mechanics guarantee that experienced players accumulate heroes indefinitely while new players have none. Hard death keeps hero count meaningful across all game lengths.
- **Makes victory against heroes feel earned.** Killing an enemy hero is a real strategic accomplishment; soft-death mechanics make it a temporary inconvenience.
- **Reinforces pillar 0.1 (realism).** Real commanders died in battle. Caesar at Munda nearly did; Gustavus Adolphus did at Lützen. Sanitizing this undercuts the simulation backdrop.
- **Reinforces pillar 0.2 (knowledge as progression).** Players learn the stakes of hero deployment by losing them. Wounded-retreat insulates from this lesson.
- **Reinforces pillar 0.6 (strategic-level decisions).** Hero placement under hard death is a real risk-vs-reward decision, not "deploy everywhere, accept downtime."

**Replaces traditional general/admiral units.** Civ-style standalone commander units are not part of the design. Heroes emerge organically from successful combat veterans, integrating veterancy and force-multiplier mechanics into one system.

**Risks:**
- New players may lose heroes to situations they didn't realize were dangerous.
- Loss of player control over hero placement (heroes emerge where battles happened, not where the player would prefer to place a general).

**Mitigations:**
- **Risk-legible UI:** predicted-outcome system clearly indicates when a hero is at significant risk before committing to combat. Verdict, not formula.
- **Explicit hero withdrawal action:** heroes can retreat from a contested area at movement or turn cost. Lets players proactively manage risk without auto-saving heroes from poor decisions.
- Heroes are mobile units; emerging in unexpected locations isn't permanent, just an initial constraint.

**Future aspects:**
- Specific specialization effects and balance tuning.
- Whether hero specialization can be retrained or is permanent.
- UI for displaying hero status, achievements, and risk warnings.

### 3.8 Rail Stacking and Traffic-Jam Mitigation

**Decision:** Higher-tier infrastructure (railroads) allows unit stacking during auto-pathfinding transit. Roads and lower-tier infrastructure remain strictly 1UPT.

**Mechanism:**
- Units stack when transiting via auto-pathfinding on rail
- Stack limit: 5 units per tile (subject to tuning; tied to rail tier or technology)
- Stacking is a *transit mode*, not a combat mode — units unstack on engagement or arrival at destination
- Manual movement onto rail-occupied tiles does not stack (preserves player intent to keep units separate)
- Stacked tiles visually distinguished (stack count indicator)

**Splash damage risk:**
- Any combat damage to a stacked tile applies to all units in the stack (scaling rule to be designed)
- Creates a meaningful tradeoff: stacked transit is fast but vulnerable
- Players learn to disperse before contested territory

**Rationale:**
- Mitigates traffic-jam problem for friendly reinforcement movement
- Rewards infrastructure investment with direct military payoff (not just economic)
- Trains players on tedium-removal tools (auto-pathfinding) by attaching benefit to it
- Logistics design pillar reinforced — rail isn't just for resources, it's strategically transformative
- Discovery via play: convenience benefit obvious immediately, splash damage risk learned through experience

**Risks:**
- Players who don't understand splash damage take catastrophic losses ("gotcha" feel).
- Infinite or large stacks could trivialize movement.

**Mitigations:**
- Visual distinction of stacked tiles is essential.
- Stack limit (5-ish) caps both the convenience and the catastrophe.
- Splash damage scaling should be predictable — single hit doesn't wipe entire stack, but damages all members proportionally.

### 3.9 Unit Upgrades vs Replacement

**Decision:** Units upgrade to newer tech (not replaced), preserving veterancy and hero status.

**Cost design (target ratios, subject to tuning):**
- Upgrade cost ≈ 70-85% of equivalent new unit's production cost
- Cost scales with veterancy: green units cheap to upgrade, veterans more expensive (retraining), heroes most expensive but always worth it
- Upgrade requires safe rear area (city or non-contested tile) and time (several turns)
- Tech-gated: can only upgrade to types currently available in tech tree
- Archetype-locked by default: melee infantry → ranged infantry possible (related archetypes), melee → cavalry not without explicit cross-training (expensive special action)

**Rationale:**
- Replacement would erase veterancy and hero progression with every tech advance — destroys the investment system
- 70-85% cost makes upgrade attractive for valuable units (veterans, heroes) but not strictly better than building new for green units
- Veterancy-scaled cost creates interesting decision: low-veterancy units might be cheaper to disband-and-rebuild; veterans worth upgrading
- Safe-rear requirement creates strategic decision (pull units from line for refit)

**Risks:**
- Cost balance is delicate. Too cheap → no reason to build new units. Too expensive → veterans get scrapped.
- Cross-archetype restrictions might frustrate players who expect to convert unit types freely.

**Mitigations:**
- Plan for tuning. Treat 70-85% as a starting range, adjust based on playtest data.
- Clear UI showing upgrade availability and cost; cross-archetype option visible but expensive (player choice, not blocked).

### 3.10 Combat Resolution (Hybrid Deterministic + Bounded RNG)

**Decision:** Mostly deterministic combat with bounded RNG concentrated at close-margin fights. No criticals, no hit/miss randomness.

**Mechanism:**

Compute strength ratio between attacker and defender after all modifiers (base stats, veterancy, terrain, height, positioning/flanking, supply state, tech, unit-vs-unit type).

- **Outside RNG band** (ratio strongly favors one side): outcome is determined. Stronger side wins, losses proportional to ratio.
- **Inside RNG band** (ratio near 1:1, working range 0.9-1.1): outcome is randomized with probability weighted by exact ratio. Genuinely uncertain fights resolved by roll.
- **Damage variance** (always applied): even certain victories have bounded damage variance (normally distributed, modest range — e.g., ±20% around deterministic expectation). Combat result unchanged; cost varies modestly.

**Band width (0.9-1.1) is the primary tuning knob:**
- Narrower band → more determinism, RNG only for truly even fights
- Wider band → more uncertainty even in clear matchups
- Default narrow; widen if combat feels too predictable. Easier to add uncertainty than remove it after expectations form.

**Predicted-outcome UI:**
- Verbal verdicts, not numerical probabilities
- "Decisive victory" / "Likely victory" / "Even fight" / "Likely defeat" / "Decisive defeat"
- Verdicts map honestly to underlying state — "decisive" really is, "even" really is uncertain
- No formulas exposed (pillar 0.2)

**Rationale:**
- Mostly-deterministic outcomes reward understanding the system (pillar 0.2)
- RNG band handles the genuine uncertainty of evenly-matched fights without making predictable fights random
- Avoiding criticals/misses prevents the "my crossbowman killed his tank" failure mode that violates the discovery pillar
- Verbal verdicts communicate uncertainty more naturally than percentages (Civ's 95%-and-lost complaint problem)
- Most fights in practice will have enough situational asymmetry (terrain, height, veterancy, supply, support) that the RNG band rarely triggers. The system feels deterministic in normal play.

**Multiplayer-safety:**
- Central combat RNG seeded at game start, threaded through all resolutions
- Deterministic resolution order when multiple fights occur on same turn
- Floating-point math discipline in combat calculations

**Risks:**
- Players reading "likely victory" as certainty and complaining about losses ("the RNG screwed me")
- Spectator legibility when an evenly-matched unit dies

**Mitigations:**
- UI language frames probability naturally; avoids numerical false precision
- Post-combat summary can indicate "this was an even fight" when appropriate

**Future aspects:**
- Specific band width tuning
- Damage variance magnitude tuning
- Resolution order rules for simultaneous combats
- Multi-unit combat prediction UI (one attacker, multiple potential defenders within range)

#### Per-edge defense (walls and natural blockers, ADR-0032)

**Decision:** Defensive modifiers attach to **tile edges**, not only to tiles. When
a unit attacks **across an edge**, that edge's defensive value feeds the defender
side of the strength ratio above. A single **shared edge-defense model** holds both
natural and built blockers in the same units:

- **Natural:** a cliff edge (ADR-0012, `edge_class` delta ≥ 2), a river edge, a
  coastline.
- **Built:** a **wall** segment (worker-built per-edge state, `workers.md` §3).

This is why settlement defense rewards terrain: a district edge already backed by a
cliff or river carries edge-defense for free, so the player walls only the open
sides. Multiple wall layers need no special rule — an attacker advancing through an
outer and then an inner walled ring simply pays the edge-defense modifier at each,
which is the "sieges are an investment" property emerging from one mechanism.

**Ancient walls are wooden.** They grant a defender bonus across the walled edge and
nothing more: **no damage/destruction model** (a wall is static edge state until its
tile is captured) and **no siege equipment** to counter them yet — both are later
eras.

**Invariant (the no-siege-counter guardrail):** because the Ancient era ships walls
without a siege counter, the wooden-wall bonus must be tuned so that a walled tile
is **always takeable by direct assault** — bloodier and slower, never impregnable.
If the bonus is set high enough to make a walled city unassailable, Ancient warfare
stalls until a later era. This is a balance dependency on the unit-strength model
(§5.4 of `known-issues.md`), not a free parameter.

**Rationale:** Unifying walls with natural blockers on one edge channel makes
"use the terrain" structural rather than an emergent hope, reuses the edge-state
machinery roads already established (pillar 0.5), and leaves a clean seam for siege
to extend later (siege acts on wall edge-state). Edge-based modifiers also slot
directly into the existing strength-ratio computation as one more defender term.

**Open questions:**
- Whether river/coast edges contribute the same magnitude as a wooden wall, or less
  (tuning).
- Stacking rule when an edge has both a natural blocker and a wall (cap, or
  diminishing); leaning "take the stronger, do not fully add" for legibility.
- Wall segment cost and bonus magnitude (tuning; the structural model is settled).



**Decision:** Three-state supply with grace-period buffers. Supply categories unlock with the units that need them, paired with required production infrastructure. Foraging mediates between in-supply and out-of-supply for eligible units.

#### Three Supply States

1. **In supply:** unit connected to supply network within range. Full effectiveness. Buffer remains full.
2. **Foraging:** unit disconnected but in hospitable conditions (suitable biome — forest, grassland, plains — in friendly/neutral territory, *and* unit type capable of foraging). Reduced effectiveness. Buffer drains slowly.
3. **Out of supply:** unit in hostile territory, harsh biomes (desert, tundra), or modern unit incapable of foraging. Significantly reduced effectiveness. Buffer drains faster. Eventually degrades to ineffective.

**Foraging eligibility by era:**
- Pre-gunpowder units: can forage in suitable terrain
- Gunpowder-era units: partial foraging (food only, not ammunition)
- Industrial+ units (motorized/mechanized): cannot forage — go directly from in-supply to out-of-supply

This produces realistic asymmetric mobility: medieval armies range further from supply than modern armies, matching historical patterns.

#### Supply Buffer Mechanic

- Each unit has a supply buffer (multiple buffers if it consumes multiple categories — see below)
- Buffer drains when not in supply, rate depends on state (foraging slow, out-of-supply faster, combat accelerates drain)
- Refills gradually when reconnected (1-2 turns to full, not instant)
- When empty, unit takes effectiveness penalties (combat strength, movement, possibly attrition)

**Rationale:** Allows rapid advances without making logistics ignorable. Quick raids within buffer range are fine; deep extended advances require infrastructure investment. Garrison forces in supply are never punished by the system — only attackers extending past it.

**Tuning:** Default 5-8 turns of full operation before degradation. Adjust based on playtest.

#### Supply Categories (Tech-Unlock Coupled)

**Decision:** New supply categories are introduced through tech progression by *coupling unit unlocks with required production infrastructure*. The unit and its supply chain unlock together as a single tech package.

**Examples:**
- Researching gunpowder unlocks **musketeers** AND **ammunition factory** (which is required to produce musketeers and to maintain their ammunition supply)
- Researching internal combustion unlocks **tanks** AND **oil refinery** (required for tank production and ongoing fuel)
- Researching aviation unlocks **aircraft** AND **aviation fuel infrastructure**

**Production cost mechanics:**
- Building a unit costs some of the supply resource (a musketeer requires X ammunition to produce)
- The cost is non-trivial but not dominant — players must invest in supply production to field these units at scale, but the supply cost doesn't overwhelm the unit's production cost

**Ongoing consumption:** Once produced, units consume their supply categories continuously. A musketeer needs food + ammunition; a tank needs food + ammunition + fuel.

**Rationale (pillar 0.2 alignment):**
- Discovery comes from committing to a tech path. Player researches gunpowder, sees both the unit and the factory unlock — understands the connection before building either.
- No "gotcha" failure mode where a player builds units without realizing they need supply infrastructure
- Each new unit type makes a clean strategic commitment visible: "want tanks? Commit to oil infrastructure and the geopolitics of oil access"
- Categories layer in over time rather than dumping complexity at once

#### Per-Archetype Consumption Profiles

| Unit type | Food | Ammunition | Fuel | Foraging? |
|---|---|---|---|---|
| Scout | low | — | — | excellent |
| Melee infantry (early) | yes | — | — | yes |
| Ranged infantry (pre-gunpowder) | yes | — | — | yes |
| Ranged infantry (gunpowder+) | yes | yes | — | partial |
| Cavalry | yes (incl. fodder) | — | — | yes |
| Siege (early) | yes | — | — | partial |
| Siege (gunpowder+) | yes | heavy | — | partial |
| Armored (modern) | yes | yes | heavy | no |
| Aircraft | yes | yes | heavy | no |
| Support (engineers/medics) | yes | — | — | yes |
| Support (logistics) | yes | — | — | yes; *boosts* nearby supply throughput |

**Implementation:** Per-unit buffer values for the categories that unit consumes (1-3 buffers). Supply network provides whichever resources flow through it. Units automatically draw from available sources. Player manages the *system* (production capacity, network coverage, security), not per-unit resource routing.

#### Pillaging (Bounded but Strong)

**Decision:** Pillaging is a real strategic tool for time-limited deep operations, not a substitute for sustained logistics. Strong enough to matter, bounded by geography and time.

**Mechanism:**
- **Pillaging recovers supply** (food, possibly ammunition for advanced eras). Refills unit supply buffers.
- **Only developed enemy tiles are pillageable** — farms, cities, infrastructure. Empty wilderness provides nothing.
- **Pillaging takes time** — unit must stay on a tile for one or more turns to extract supply. Trades mobility for sustainment.
- **Pillaging depletes the tile** — once pillaged, that tile provides nothing further for many turns.
- **Pillaging damages enemy economy** — pillaged tiles produce less for their owner over many turns afterward. This makes pillaging a weapon against enemy economy as well as a sustainment tool.
- **Pillaging has diplomatic consequences** — worsens relations, may trigger third-party intervention, increases enemy resolve.

**Rationale:**
- Historically accurate (Sherman's March, Mongol doctrine — pillaging was load-bearing for limited operations)
- Provides real strategic option without making sustained logistics optional
- Asymmetric: cavalry raids deep into enemy farmland can sustain themselves; occupying enemy territory long-term cannot rely on pillaging because tiles deplete

**Risks:**
- Pillaging being too strong → encourages sustained marauding instead of conventional warfare
- Pillaging being too weak → reduces to a forgettable UI button (the failure mode in most 4X games)

**Mitigations:**
- Depletion + recovery time prevents permanent sustainment from pillaging
- Diplomatic consequences add costs beyond the immediate gain
- Tuning is delicate; expect to iterate

**Future aspects:**
- Specific extraction rates, depletion duration, recovery time
- Whether pillaging produces resources for the pillaging player or only sustains their units
- AI doctrine for when pillaging is appropriate

#### Supply Line Vulnerability and Reconstruction

**Decision:** Infrastructure damage and rebuild rates form a gradient. Most damage is recoverable; bridges and tunnels are decisive.

**Reconstruction rates:**
- **Roads:** quick to repair (1-2 turns), low resource cost
- **Railroads:** moderate to repair (several turns), moderate resource cost
- **Bridges:** slow to rebuild (many turns), high resource cost
- **Tunnels:** very slow to rebuild (longest), highest resource cost

**Demolition mechanics:**
- **Destruction is a deliberate action, not a free side-effect.** Units cannot destroy infrastructure by simply moving onto it.
- **Engineering effort required.** Demolition takes engineering-capable units, multiple turns of dedicated work, and being uninterrupted during the operation.
- **Defenders can scorched-earth their own infrastructure** but it's costly — same engineering effort, same time investment.
- **Bridge/tunnel destruction is particularly heavy.** Significant engineering operation, vulnerable to interception during the demolition turns.

**Rationale:**
- Easy-to-rebuild roads keep raiding meaningful without making war attritional
- Hard-to-rebuild bridges/tunnels make rivers and mountain passes decisive terrain (Verdun, Ardennes pattern)
- Deliberate demolition cost prevents spam destruction tactics
- Scorched earth cost prevents trivial defender denial — must be a real strategic decision

**Second-order consequence:** Strategic geography of war is dominated by chokepoints (major rivers, mountain passes). Maps without major chokepoints play differently from chokepoint-heavy maps. The generation pipeline produces chokepoint geography naturally via canyons and rivers — desirable interaction.

**Future aspects:**
- Specific repair times and resource costs per infrastructure type
- Engineering unit specifications and demolition mechanics
- Interception rules during demolition operations
- Whether some critical infrastructure (major dams, large bridges) requires multiple engineering units

### 3.12 Combat Topics Not Yet Designed

Tactical model, unit interface, scale/representation, archetypes, veterancy/heroes, rail stacking, upgrades, combat resolution, and supply/logistics all settled. The following remain open:

- **City capture mechanics:** previously flagged. Does the whole city footprint fall on capture, or can individual developed tiles be captured/pillaged? Can sieges target supply networks rather than the city directly? Interacts strongly with supply line mechanics now designed.
- **Frontage and flanking mechanics:** attacking from multiple sides, formations.
- **Naval combat:** separate system or unified with land combat. Interacts with map shape (archipelago vs. continents).
- **Air combat:** if included at all, when in the tech tree, and how integrated.
- **Hierarchical orders (v2+ consideration):** designate one unit as commander, others follow. Adds tactical depth beyond batch commands. Not v1.

### 3.13 Unit Composition: Chassis and Components

**Decision:** A unit is a **base chassis** plus equippable **components**
(weapons, armor; more component classes in later eras). The chassis is the unit
type — it defines the archetype and role (§3.6) and is always buildable from
base-tier materials. Components are the resource-gated augments: they scale a
unit's magnitude and add situational modifiers *within* its archetype, and they
are where higher-tier resources (bronze, then iron, etc.) attach to the unit
model. Research of incremental same-archetype units (Warrior → "better melee" →
"better melee again") is replaced by research of weapons and armor lines.
Research of new *types* (e.g., siege in the Classical era) remains.

**Invariants.**

- **Chassis is the critical path.** Every archetype's base chassis is buildable
  without higher-tier resources. Components gate strength and efficiency, never
  progression. This is the unit-level expression of the §2.7 / §1.16 resource
  gate: an unlucky start with no copper fields is weaker, not stuck.
- **Components tune within an archetype, never across it.** Armor makes a melee
  unit a tankier melee unit; no loadout may let a unit defeat its soft-RPS
  counter (§3.6). Role comes from the chassis; components scale magnitude and add
  situational modifiers inside that role. This is a hard invariant, not a tuning
  target — if a loadout can cross the triangle, composition stops mattering and
  §3.6 collapses.
- **Customization is intent-level (§0.6).** The player designs a **loadout
  template**; units are built to the active template; doctrine can set templates
  army-wide (§3.3). The player never kits individual units by hand. Manual
  per-unit equipping is the override, not the primary interface — the same
  manual-as-override pattern as §2.7 allocation and §3.3 standing orders.
- **Re-equipping is the upgrade mechanism (§3.9).** Swapping components onto an
  existing unit is how units advance, preserving veterancy and hero status
  rather than replacing the unit. Re-equip happens at a city or within supply
  range (§3.11), not free in the field.

**Ancient-era scope (ADR-0031).** Base chassis are **warrior** (melee) and
**archer** (ranged), plus the non-combat worker, scout, and settler — **not**
"warrior, spearman, archer." The era's single component slot is the **weapon**,
and its augment tier is **bronze** (copper-gated, `resources.md` §3). The
**spear is the Ancient melee weapon component**: a warrior equipped with the
bronze spear surfaces as the named unit **"Spearman."** This is the chassis model
working as designed — "spearman" is a *named loadout*, not a separate chassis — and
it generalizes cleanly: the Classical iron **sword** on the same warrior chassis
surfaces as **"Swordsman."** Bronze is the era's defining material, so "warrior or
archer chassis + one bronze weapon" is both the simple system the era wants and
thematically exact. The full multi-slot component designer blooms in the Classical
era and later.

- **Archers gate behind a building, not the center.** The warrior, worker, scout,
  and settler come from the city center's bootstrap build-slot (`cities.md` §2.7);
  the **archer chassis is unlocked by the encampment** (a military-district
  building, `ancient-content.md`), giving the military district an early reason to
  exist.
- **Ancient unit upkeep is food.** Units draw a continuous food upkeep from their
  network (`resources.md` §9), tying army size to the agricultural base; the
  barracks building reduces it. Field-supply states are §3.11.

**Rationale.** Collapses an N×M redundant tech ladder (every archetype × every
strength tier) into orthogonal component lines researched once — fewer techs,
more meaningful ones, and it is the natural home for the §2.7 resource gate.
Re-equip-as-upgrade reuses §3.9 rather than adding a parallel mechanism (§0.5).
Intent-level loadouts head off the per-unit kitting tedium that forced
auto-design buttons in comparable games — the §0.6 failure mode, anticipated
rather than patched.

**Risks.**

- Per-unit kitting tedium if the model is built manual-first and templates are
  retrofitted.
- A loadout that crosses the archetype boundary silently breaks soft-RPS.
- Legibility loss. A named ladder (Warrior → Swordsman → Legion) tells the player
  what beats what at a glance; "bronze weapons, tier 2" is more abstract and a
  less visceral unlock reward. Pandora-style abstraction loses players who want
  the named milestone.

**Mitigations.**

- Design the template/doctrine layer as the primary interface from the start;
  manual equipping is the override.
- The within-archetype invariant is enforced in the model, not left to tuning.
- **Name loadouts** so research still delivers a *named unit*: a bronze spear on
  the warrior chassis surfaces as a **Spearman**, an iron sword (Classical) as a
  **Swordsman** — a named milestone, not a stat delta — while the underlying
  structure stays orthogonal components on a fixed chassis.

**Open questions.**

- Component slot count per era (ancient = 1; later multi-slot). Tuning.
- Whether component research lines (weapon, armor) are fully orthogonal to type
  research or interleaved. Leaning orthogonal.
- Exact re-equip cost and the city-vs-supply location rule — pin alongside the
  supply detail in §3.11.
- This partially pre-designs the (still-undesigned) tech tree: techs now unlock
  *types plus components*, not unit tiers. Flagged so the tree stays consistent
  when it's designed.
