# Strategy Game — Known Issues, Open Topics, and Risks

---

## Open Topics — Roadmap

### Map generation design: COMPLETE

All six pipeline stages designed (tectonics → elevation → climate → hydrology → biomes → resources → starting positions) plus three-layer player control. Remaining decisions are tuning-level and resolvable during implementation. See `worldgen.md` §1.16.

### Next phase: implementation infrastructure

Before implementing any generation stage, build the supporting infrastructure:

1. **Shared `coords` module** — wrap-aware coordinate math, used by both generation and runtime.
2. **Debug visualization tooling** — PNG dumps of intermediate fields (heightmap, plates, temperature, moisture, flow accumulation, biomes, resources, starting positions). Load-bearing for parameter tuning. Build before first real stage implementation.
3. **Determinism test scaffold** — verify same seed produces byte-identical `WorldData` across machines and across runs.
4. **Standalone CLI runner** — produce a map and dump PNGs without Bevy. Enables fast iteration (seconds, not game-start times).

After infrastructure, implement stages in pipeline order, visually validating each before moving on.

### City/gameplay topics

5. **Combat — substantially designed.** Tactical model, batch commands, standing orders, strategic-level focus, unit scale (company, 1UPT), archetypes (soft RPS), veterancy with hard-death heroes, rail stacking with splash damage, upgrade mechanics, combat resolution (hybrid deterministic + bounded RNG), and supply/logistics (three-state with tech-unlock-coupled categories, pillaging, infrastructure vulnerability gradient) all settled. Remaining: city capture, frontage/flanking, naval, air, hierarchical orders (v2+). See `combat.md` §3.12.
6. **City management tedium / production directives** — apply the intent-level pattern (pillar 0.6) to city production. Continuous-production directives, doctrine settings, automatic delivery. Same UI metaphors as combat batch commands. See `cities.md` §2.11.
7. **Interactions between the four growth dimensions** — does footprint gate population, does development gate specialization, etc. Population now explicitly gates promotion (and via that, district radius); the assumption that population drives footprint growth is working but not locked. Whether development can outrun population needs a deliberate decision. Foundational — blocks city implementation.
8. **Production/allocation model** — queues vs allocation vs job slots. Interacts with item 6. Foundational — blocks city implementation.
9. **Specialization mechanics** — partially resolved. Specialization is chosen at outpost founding via the center building and can be reconfigured by replacing it (see `cities.md` §2.12). The remaining open question is whether specialization should also be influenced by surrounding geography (e.g., automatic specialization boosts when founded on ideal terrain), or whether the player choice is the only driver.
10. **Migration system** — if population can move between settlements (recommended), the rules and infrastructure for it.
11. **Emergent vs declared regions** — final decision. Foundational — affects infrastructure design, UI design, and trade flow implementation.
12. **Infrastructure mode set** — roads, rails, ports, and their tradeoffs.
13. **Unfed city behavior** — refined and partially resolved: outposts handle the "minimum-persistence" case naturally; cities that lose supply lose population back toward outpost tier but do not auto-demote. Player chooses whether to demote-and-rebuild or hold and rebuild supply. See `cities.md` §2.4.
14. **Capital mechanic** — keep mobile/descriptive (current direction) or revisit.
15. **UI/legibility systems** — dashboards, alerts, overlays. Non-trivial dev budget. Especially important given pillars 0.2 (knowledge as progression) and 0.6 (intent-level specification).
16. **Tech-unlock traversal of impassable terrain** — bridges (edge-based), tunnels (tile-based), mountain passes. Data model decisions deferred until implementation.
17. **Founding mechanism details** — whether the settler unit costs empire population (Civ 6 model), simply costs production, or uses a hybrid. The §2.4 revision keeps founding cheap; this open topic is the specific cost shape.
18. **Growth-accelerating building category** — cathedral-style buildings that boost population growth need a unified category design (rate boost vs. capacity boost, prerequisites, per-empire vs. per-settlement). Deferred until later eras are designed.
19. **Claim growth rate and direction** — the specific rate at which population thresholds unlock new claim rings, and whether specialization-driven auto-claim direction is deterministic or weighted.

### Implementation-level decisions deferred from map generation

20. Default map size (working: 240×150).
21. Specific resource list and rarity tuning.
22. Biome lookup table specifics.
23. Advanced layer final slider set.
24. Adequacy floor thresholds for starting position viability.
25. Alpine zone as biome or feature.
26. Forest density as separate feature pass.

---

## Known Issues, Risks, and Acknowledged Tensions

Issues identified during design review that must be tracked, stress-tested, or accepted explicitly. Distinct from "open topics" (which are undesigned) — these are problems with the current design that need ongoing attention.

### 5.1 Late-Game Turn-Time Performance (Major Risk)

**Concern:** Many systems independently push toward heavy per-turn computation:
- Climate advection at generation (less critical — runs once)
- Supply network resolution with multi-resource flow
- Per-tile economic dynamics for cities
- Combat with stacked modifiers (terrain × height × veterancy × supply × support × tech × unit-vs-unit)
- AI thinking at force level across many forces (§3.4)
- Standing orders re-evaluated every turn for every unit (§3.3)
- Region emergence from infrastructure topology (§2.9)
- Multi-tile city updates (§2.1, §2.6)

Each is reasonable in isolation. Together, late-game turn processing could become unacceptably slow — *especially* in the late game where tedium-reduction tools are most needed and where the player base is most likely to abandon the game. Civ 6 famously has minutes-long late-game turns; this design has more per-turn computation than Civ 6.

**Status:** **Major flagged risk.** Not solved by any single decision. Must be stress-tested continuously as systems are implemented.

**Required discipline going forward:**
- Establish explicit turn-time budgets (e.g., "late-game turn must complete in under N seconds at default map size with default player count"). Without a budget, scope creep is invisible.
- Profile aggressively from the first integrated implementation onward, not at the end.
- Treat performance regressions as bugs, not "we'll optimize later."
- Design new systems with computational cost in mind, not as an afterthought.
- Stress-test at scale (max map size, max player count, late-game saves) regularly.

**Possible mitigations to keep in mind:**
- Multithreading systems that don't have ordering dependencies (carefully, given pillar 0.4 multiplayer safety — parallelism must remain deterministic)
- Incremental computation (recompute supply only on network change, not every turn)
- Hierarchical pathfinding (HPA*) for unit movement
- Spatial indexing for range queries
- AI decision caching where strategic situations haven't changed

**Why this is flagged as "major":** Performance failure modes are hard to reverse late in development. A game that ships with 5-minute late-game turns is functionally broken regardless of how good the design is. This risk should be present in every implementation review.

### 5.2 Multiplayer Knowledge Asymmetry (Accepted)

**Concern:** Pillar 0.2 (knowledge as progression) and pillar 0.4 (multiplayer-safe by default) interact in a way that creates large skill gaps in multiplayer based on systems knowledge, not just decision quality. An experienced player who has internalized climate, supply, combat resolution, and economic patterns will systematically dominate new players in ways that aren't really about play skill.

**Resolution: accepted as design.** Experienced players dominating newer players is genre-standard and expected. The discovery-based progression makes the magnitude of advantage larger than in Civ-style games where mechanics are exposed, but this is acceptable.

**Acknowledged consequence:** Multiplayer matchmaking and mode design (when those systems are eventually designed) will need to account for the *magnitude* of knowledge advantage being larger than genre norm. Possible considerations for later:
- Skill-based matchmaking that accounts for systems knowledge, not just win rate
- Tutorial campaigns or single-player content that teaches systems before multiplayer
- Optional "competitive" multiplayer mode with more transparency (not committed; flagged for consideration)
- Community guides and wikis as part of the intended onboarding path (consistent with pillar 0.2)

**Why this is flagged but not "major":** It's an accepted property of the design, not a flaw to fix. But it needs to be remembered when multiplayer modes are designed in detail — defaulting to "throw everyone together" will produce poor multiplayer retention.

### 5.3 Deferred Decisions That Are Actually Foundational

**Concern:** Several items in the "future aspects" / "open topics" lists are not really deferrable — they block downstream design or implementation. Distinguishing "tabled for priority" from "tabled because answer unclear" matters for planning.

**Items now resolved (no longer blocking):**
- **Tile ownership granularity (§2.2)** — superseded by `cities.md` §2.14 (empire-level ownership) and the §2.13 district-vs-claim distinction. The original "core / developed / influenced" three-tier proposal is replaced by "district tile / claimed-and-worked tile / claimed-but-unworked tile."
- **Expansion vs Development (§2.8)** — now has a concrete spatial mechanism via `cities.md` §2.13. Expansion claims tiles; development places districts.

**Items that still block downstream work:**
- **Interactions between the four growth dimensions (§2.6)** — city design has structural holes until this is decided. Does footprint gate population? Does development gate specialization? Can't fully implement city mechanics without answers.
- **Production/allocation model (§2.7)** — section §2.11 (city management tedium) prescribes solutions that assume a production model. Can't be implemented without it.
- **Emergent vs declared regions (§2.9)** — affects infrastructure design, UI design, and trade flow implementation.
- **Specialization mechanics (open topic)** — interacts with city design, AI city management, and the four-dimensions question.

**Required action:** When implementation begins for city systems, these decisions must be resolved first. They are not optional refinements.

**Items that can genuinely wait:**
- Capital mechanic (descriptive only is fine for v1)
- Tech-unlock traversal of impassable terrain (only matters when those techs exist)
- Migration system (additive feature, not foundational)
- Naval/air combat (large but independent systems)

### 5.4 Unit Balance and Combat Validation

**Concern:** Combat sections §3.5–§3.11 design mechanisms in detail but never address the foundational question of *what a unit's combat strength is and how it scales across tech eras*. Several mechanisms depend on this:
- Hard-death heroes (§3.7) assume heroes are meaningfully stronger than green units
- Upgrade-vs-replacement (§3.9) assumes upgrade cost is calibrated to combat value
- Combat resolution (§3.10) assumes strength ratios are meaningful and tunable

**Status:** Cannot validate the combat design without rough balance numbers, even placeholder values. "Tuning" is treated as separate but some tuning affects whether mechanics work at all.

**Required action:** Before deep implementation of combat systems, establish a basic unit-strength model — even placeholder values — and verify that the designed mechanics produce sensible outcomes within that model. Iterate from there.

### 5.5 Standing Orders Default Behavior for Heroes (Minor Tension)

**Concern:** Pillar 0.6 (intent-level specification) drives standing orders toward automation. Hard-death heroes (§3.7) require deep player attention to placement and risk. These are in mild tension — standing orders for heroes could lead to careless hero deaths.

**Resolution:** Hero units should have more conservative standing-order defaults than regular units. More alerts, more frequent player intervention requests, possibly an explicit "this is a hero, are you sure?" affordance when committing to risky orders. Standing orders on heroes are still allowed (the player retains control) but defaults should err on the side of preserving heroes.

**Status:** Minor design constraint to remember when standing orders are implemented in detail.

### 5.6 Narrative Texture (Future Pillar Candidate)

**Concern:** The design organically produces narrative gameplay moments — heroes emerging from battle veterans, Sherman-style pillaging raids, supply collapse during overextension, geographic monopolies driving wars, chokepoint battles at canyons and rivers. These emergent narratives are a real value of the design but are not currently stated as a pillar.

**Status:** Flagged for consideration as a future pillar (or value statement). If made explicit, future design decisions can deliberately preserve and amplify narrative texture. If not made explicit, future decisions might inadvertently flatten it.

**Worth considering when:** UI design, post-game summaries, or event/notification systems are designed. These are the places where emergent narrative is either surfaced (making it visible to players) or hidden (existing but unnoticed).

### 5.7 Player Count and Map Size Scaling

**Concern:** Advanced layer parameters include both map size and number of players (`worldgen.md` §1.15). The relationship between them is unspecified. Edge cases — 12 players on a small map, 2 players on a huge map — need defined behavior.

**Required decisions:**
- What's the relationship between map size and supported player count? Hard limits, soft warnings, or unrestricted with consequences?
- When starting position adequacy can't be satisfied for N players, what happens? The doc says "fail honestly" but doesn't specify behavior — refuse to generate? Generate with warning? Reduce player count automatically?

**Status:** Minor but needs resolution before generation pipeline is exposed to the player. Tractable; just needs explicit design.

### Others

- **Gold has no sink.** Gold accumulates empire-wide and is yield-only; nothing
  spends it yet (`resources.md` §6). The sink side — gold-payable maintenance,
  unit upkeep, purchasing — is undesigned. The accumulator looks complete and is
  not. Resolve when the treasury/economy sink system is designed.
- **Amenity system is structurally complete but untuned.** Per-luxury amenity
  amount, demand-per-population curve, and growth bonus/penalty magnitudes are
  placeholders (`resources.md` §5). Deficit penalty stays weak until later eras.
- **Worker / tile-improvement verb set undesigned.** It is the deposit-to-yield
  bridge (`resources.md` §7); until designed, no deposit produces a yield. Next
  system in the dependency chain.
