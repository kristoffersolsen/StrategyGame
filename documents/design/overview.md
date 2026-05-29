# Strategy Game — Design Overview

Design pillars and cross-cutting principles. These are the load-bearing commitments that shape all downstream decisions. When a specific design choice is unclear or contested, refer back to here.

---

## 0. Design Pillars

### 0.1 Realism as Foundation, Not Foreground

The simulation backbone (geology, climate, hydrology, economy, supply networks) provides the texture and consequences of the world, but is not directly exposed to the player. Mechanics are surfaced through their effects, not their formulas. Players should *feel* the world's realism — coastal regions are wetter, mountains shape weather, cities depend on supply — without being lectured about it.

### 0.2 Knowledge as Progression

**The player's understanding of the game's systems is itself the primary progression curve.** New players see consequences without causes — events feel almost random. As they play and experiment, they begin to connect patterns to systems, and eventually internalize the simulation logic underneath. Mastery is defined as systems-comprehension.

Implications:
- **Feedback at the level of consequence, not mechanism.** UI shows "city is in unrest due to housing shortage" — not "happiness < threshold T because population × demand_coefficient > housing × capacity_factor."
- **Counterintuitive outcomes are acceptable if consistent.** Player loses an army unexpectedly? They should ask "what didn't I account for?" not blame RNG. Consistency in underlying systems makes failure educational.
- **Discovery is the design target.** Players figuring out *why* spices cluster on certain continents, *why* certain terrain combinations matter, *why* large cities have characteristic problems — this is the experience to optimize for.
- **No intrusive tutorialization.** Tooltips explain *what* (numbers, immediate effects). They do not explain *why at the simulation level*. The why is to be discovered.
- **Externalize the explanation layer to community.** Wikis, forums, and player guides become the venue for system-understanding. This is desirable — it builds community, retention, and shared mastery culture without cluttering the game. Design simulation models to be coherent and consistent enough that reverse-engineering is rewarding.

### 0.3 Player Choice as Replayability

Three-layer parameter exposure (Basic / Advanced / Full Tunability) serves distinct audiences without overwhelming any. Full Tunability uses precise technical terminology — the complexity barrier is itself the gate; patronizing accessibility-renaming would harm the audience that uses it without protecting anyone else.

### 0.4 Multiplayer-Safe by Default

All systems are designed to be deterministic and network-replicable from the start. Command layer + PlayerId + NetworkId architecture is non-negotiable. No system, however small, is allowed to introduce non-determinism (thread-local RNG, HashMap iteration order, parallel reductions without ordering).

### 0.5 Doing Things Correctly From the Start

Idiomatic framework usage, sound architecture, and proven patterns from day one — without over-engineering or premature abstraction. When evaluating proposals, the test is not "can this work" but "is this the right way to do this." Shortcuts that compromise foundational quality are not accepted, even when faster.

### 0.6 Specify Intent at the Level Where Intent Changes

A cross-cutting design pattern: as the game progresses, the number of entities the player manages grows (units, cities, trade routes, tiles), but the *interesting decisions per entity* don't grow proportionally. Most entities most turns should be doing something routine. Forcing manual per-entity specification of "routine" is tedium without depth.

**Principle:** the player should specify intent at the level where intent changes, not at the level where execution happens.

- Intent at army/front level → execution at unit level (standing orders, batch commands)
- Intent at empire/region level → execution at city level (production directives, doctrine settings)
- Intent at strategic-priority level → execution at trade/supply level (priority rules, automatic routing)
- Intent at specialization level → execution at tile level (city specialization drives tile development)

This pattern applies consistently across systems. Same UI metaphors, same alert system for "your standing directive can't be fulfilled," same override semantics (player can always do something manually instead). These are quality-of-life tools, not gameplay shortcuts — the player retains full strategic control; only mechanical execution is automated.

### 0.7 Tension Between Pillars

These pillars are mostly aligned but can pull against each other:

- **Realism (0.1) vs Knowledge as Progression (0.2):** Realism wants the world to be coherent and detailed; discovery wants the systems to be revealed gradually. Resolution: simulation runs fully, but UI surfaces only consequences.
- **Realism (0.1) vs Player Choice (0.3):** Some Full Tunability parameter combinations break realism. Resolution: Full Tunability accepts this; the audience self-selects for experimentation.
- **Knowledge as Progression (0.2) vs Player Choice (0.3):** Full Tunability exposes mechanisms directly, bypassing discovery. Resolution: accepted — the audience that uses Full Tunability has self-selected out of the discovery progression.
- **Intent-Level Specification (0.6) vs Knowledge as Progression (0.2):** Automation systems must be discoverable in behavior, not tutorialized. Players should learn what directives are effective by trying them, not from explanations.

When pillars conflict in a specific design decision, the resolution should be documented in that decision's section.

---

## Cross-Cutting: Determinism and Multiplayer Safety

**Decision (carried forward from prior work):** All systems must be multiplayer-safe by design. Command layer, `PlayerId` on all actions, `NetworkId` on all entities. Map generation must be deterministic.

**Implications for everything:**
- All RNG explicitly seeded and threaded.
- No iteration over non-deterministically-ordered collections in any system that affects game state.
- City growth, resource flow, AI decisions — all must be deterministic given identical inputs.
