# Strategy Game — Design Decisions Log

A living document capturing decisions made, their rationale, risks, mitigations, and open questions. Update as the design evolves.

---

## 0. Design Pillars

These are the load-bearing design commitments that shape all downstream decisions. When a specific design choice is unclear or contested, refer back to these.

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

## 1. Map Foundation

### 1.1 Tiling
**Decision:** Hexagonal tiling, flat-top orientation, axial coordinates.

**Rationale:** Hex is the genre standard for good reasons (uniform neighbor distance, better movement feel). Flat-top fits east-west wrap naturally — "east" and "west" are unambiguous directions. Axial storage with on-demand conversion to cube for distance/LOS math.

**Risks:**
- Flat-top is less common in popular Civ-likes (Civ uses pointy-top), so reference material may need adaptation.

**Mitigation:**
- Red Blob Games' hex reference covers both orientations. Encapsulate coordinate math in one module so the choice is contained.

### 1.2 Wrap-around
**Decision:** Cylindrical wrap east-west, no wrap north-south.

**Risks:**
- Pathfinding, range queries, and AOE calculations must all be wrap-aware. Easy to forget, hard to debug retroactively.

**Mitigation:**
- Bake wrap-awareness into the coordinate distance function from day one. Normalize q-coordinate modulo map width in all spatial queries. Write unit tests covering wrap edge cases early.

### 1.3 Tile Composition
**Decision:** Tiles are composed of independent components, not a single enum:
- **Terrain** (base type — plains, grassland, desert, ocean, coast, lake, tundra)
- **Feature** (forest, marsh, hills, floodplains, etc.)
- **Resource** (iron, wheat, etc.)
- **Occupying structure reference** (optional reference to a city/fort entity that occupies this tile)

Initial terrain set: plains, grassland, desert, ocean, coast, lake, tundra. Open for expansion.

**Rationale:** Flat enums force `match` statements across the codebase; every new terrain becomes a multi-file change. Component composition + data-driven properties scales cleanly. Buildings/cities are separate entities that *reference* tiles, not components on tiles, to avoid duplicated state.

**Risks:**
- Data-driven terrain properties require a config loading system (likely RON) before the foundation is complete.
- Component proliferation can hurt query ergonomics if not designed carefully.

**Mitigation:**
- Start with a `TerrainProperties` resource as an interim step if RON loading isn't ready. Migrate to data-driven later.
- Keep components small and orthogonal. Use Bevy's `Bundle` patterns for common combinations.

### 1.4 Height
**Decision:** Discrete height levels. From lowest to highest: ocean → coast → 3 traversable land levels → mountains.

**Cliff impassability:** Height delta ≥ 2 between adjacent tiles is **impassable by default** (cliffs). Mountains are impassable. Canyon walls (produced by hydrology) are impassable.

**Late-game tech unlocks** (deferred design) will allow traversal:
- Bridges (edge-based feature, cross specific canyon/cliff edges; destructible in war)
- Tunnels (likely tile-based feature, makes specific impassable tile traversable)
- Mountain passes / engineering for delta-1 cliff smoothing

**Rationale:** Discrete is standard, far simpler than continuous, and easier to render visually. Three traversable levels is the working compromise. Impassable-by-default makes terrain strategically meaningful in early game and gives engineering tech a payoff later. Destructible bridges create good gameplay (chokepoints restored in wartime).

**Risks:**
- Height affects movement, combat, visibility, rendering, and pathfinding simultaneously. Each system needs to handle it consistently.
- More levels = more cliff edges = more "why can't I walk here" friction.
- Edge-based features (bridges) are a new data category not yet needed elsewhere.

**Mitigation:**
- Pathfinding cost function takes height delta as input from the start; treats delta ≥ 2 as infinite cost unless a traversal feature is present.
- Visual differentiation between levels must be unambiguous; this is a rendering requirement, not polish.
- Start at 3 traversable levels; only expand if playtesting shows meaningful differentiation between them.
- Defer edge-feature data model design until bridges are actually being implemented; ensure rivers (also somewhat edge-like in real geography but modeled as tile features here) don't accidentally force a premature decision.

### 1.5 Rivers
**Decision:** Rivers as a tile property (or thin river-tiles), not edge-based. Enabled by the smaller-tile model.

**Rationale:** Edge-based rivers (Civ's approach) are necessary when tiles are large because rivers would otherwise occupy entire hexes. With smaller tiles, rivers can naturally meander across multiple tiles, giving a more organic result and avoiding a separate edge data structure.

**Risks:**
- Hydrology that looks right (rivers flow downhill, accumulate, reach the sea) is non-trivial to generate.
- Player expectations from other Civ-likes may conflict with the different visual/mechanical role of rivers here.

**Mitigation:**
- Hydrology via flow accumulation from the heightmap during generation. User's numerical/modeling background is well-suited to this.
- Treat as a generation-time concern; rivers are baked into tile data once generated.

### 1.6 Tile Scale
**Decision:** Smaller tiles than typical Civ-likes. Cities span multiple tiles, units move across more tiles per turn, map feels more detailed without feeling larger.

**Rationale:** Cramped, abstract feel of conventional 4X maps is something the project explicitly aims to improve. Smaller tiles enable better-looking maps, more tactical movement, organically shaped cities, and natural-feeling rivers.

**Risks:**
- **Rendering load:** ~9× more tiles than conventional (e.g., 240×150 ≈ 36,000 tiles vs 80×50 ≈ 4,000). Naive entity-per-tile rendering will not scale.
- **Pathfinding cost:** A* on 36K nodes is meaningfully more expensive, especially with many simultaneous units.
- **UI scaling:** movement previews, attack ranges, ZOC highlights all cover much larger areas.
- **AI complexity:** strategic reasoning over a denser map is harder.
- **Game pacing:** more tiles can amplify lategame slog.

**Mitigation:**
- Commit to instanced/batched rendering from day one — one mesh, instanced per tile with per-instance data.
- Plan for hierarchical pathfinding (HPA* or similar) earlier than a conventional Civ-like would need it.
- Parameterize tile-to-meaningful-unit ratio (`TILES_PER_REGION`, movement points per turn) so gameplay numbers are tunable independently of map resolution.
- Validate the rendering approach with a stress test (e.g., 40K tiles rendered, panning at 60fps) before locking in.

**Future aspects:**
- Define exact target map size and tile count.
- Build the 40K-tile rendering stress test.
- Choose pathfinding strategy (flat A*, HPA*, flow fields) based on profiling.

### 1.7 Map Generation
**Decision:** Seeded procedural generation, fully deterministic. No handcrafted maps.

**Rationale:** Replayability, leverages user's numerical modeling background, deterministic generation is required for multiplayer-safe foundations.

**Risks:**
- Non-determinism creeps in easily (HashMap iteration order, thread-local RNG, floating-point order-of-operations).
- Procedural generation that looks structureless is the default failure mode.

**Mitigation:**
- Single seeded RNG (`rand_chacha`), threaded explicitly through generation. No `thread_rng()` anywhere.
- Use `BTreeMap` or sort-before-iterate for any iteration that affects generation.
- Test determinism explicitly and early — same seed must produce byte-identical maps across machines.
- Pipeline structure detailed below.

---

## 1.8 Map Generation: Architecture

**Decision:** Generation runs at runtime, on game start.

**Rationale:** Player-tailorable experience requires fresh maps per game.

**Time budget:** 15–30 seconds for default map size. Each stage gets a slice of this budget. Stages exceeding their slice get optimized or simplified.

**Threading:** Generation runs on a background thread via Bevy's `AsyncComputeTaskPool`. Main thread shows a progress UI updated via a channel (`crossbeam-channel` or `std::sync::mpsc`) carrying `GenerationProgress` events from the background thread.

**Module structure:** Generation is a **separate Rust module**, not a set of Bevy systems. Produces a finalized `WorldData` structure that is then spawned into Bevy entities by a single boundary system on the main thread.

```
[background thread]
generate_world(seed, params) -> WorldData
    ├─ generate_tectonics(...) -> Plates
    ├─ generate_elevation(plates, ...) -> HeightMap
    ├─ generate_climate(heightmap, ...) -> Climate (temp + moisture)
    ├─ generate_hydrology(heightmap, climate, ...) -> Rivers + Lakes + Canyons
    ├─ generate_biomes(climate, heightmap, hydrology, ...) -> BiomeMap
    ├─ generate_resources(biomes, ...) -> Resources
    └─ generate_starting_positions(...) -> StartPositions

[main thread, after generation completes]
spawn_world_from_data(world_data) -> spawns tile entities, structures, etc.
```

**Why separate module, not Bevy systems:**
- Generation is sequential one-shot, not per-frame parallel
- Intermediate data (plates, raw heightmaps) shouldn't pollute the ECS after generation
- Pure-Rust generation is trivially testable in isolation
- Can be run as a standalone CLI for debugging ("generate this seed, dump to PNG")
- Coordinate/wrap logic lives in a shared `coords` module used by both generation and runtime — no duplication

**Risks:**
- Generation time exceeding budget at the default map size.
- Determinism breakage across stages (floating-point order, parallel reductions, etc.).
- Progress reporting boilerplate adding noise to each stage.

**Mitigations:**
- Profile each stage. Build the debug visualization tooling (PNG dumps of intermediate fields) early — it's load-bearing for both debugging and parameter tuning.
- Determinism testing as part of the test suite: same seed must produce byte-identical `WorldData` across machines.
- Wrap progress reporting in a small helper so individual stages stay clean.

**Player-exposed parameters (minimal+ approach):**
- Map size
- Land/water ratio (sea level)
- Map shape (continents / archipelago / pangaea / islands)
- Climate band (cold / temperate / hot — shifts global temperature)
- Possibly: river density

Internal parameters (plate count, wind speeds, advection iterations, etc.) are fixed or weakly randomized, not exposed.

**Future aspects:**
- Lock in default map size (drives the time budget; current working range: 240×150 ≈ 36K tiles).
- Build the debug visualization tooling before the first real generation work.

---

## 1.9 Stage 1: Tectonics and Base Elevation

**Decision:** Faked tectonics — geological-looking output without simulating geological history.

**Algorithm:**
1. Generate 8–12 plate seeds via Poisson disk sampling (wrap-aware)
2. Voronoi partition with low-frequency noise-perturbed boundaries (avoids artificial straight lines)
3. Assign plate type (continental/oceanic) targeting desired land ratio (~40–50%)
4. Assign plate motion vectors (random direction, magnitude from distribution)
5. Classify boundaries by relative motion: convergent / divergent / transform
6. Compute base elevation from plate type (continental ~positive, oceanic ~negative, with intra-plate noise)
7. Apply boundary modifications via lookup table:
   - Continental-continental convergent → high mountains
   - Continental-oceanic convergent → coastal mountains + offshore trench
   - Oceanic-oceanic convergent → island arc + trench
   - Continental-continental divergent → rift valley
   - Oceanic-oceanic divergent → mid-ocean ridge
   - Transform → cosmetic only
   - Falloff with distance from boundary (Gaussian or similar)
8. Overlay multi-octave noise for fine detail (added on top of tectonic structure, not used to generate it)
9. Apply sea level via target-ratio quantile threshold
10. Map continuous elevation to discrete height levels (ocean, coast, 3 traversable land, mountains) — quantile-based bands

**Rationale:** Plate-based generation produces geologically coherent features (linear mountain chains, island arcs, paired trench-mountain systems) that pure noise cannot. Faked tectonics gets ~90% of the visual benefit at ~5% of the cost of real simulation.

**Risks:**
- Plates too uniform → homogeneous-looking map.
- Boundary effects too sharp → mountains look like walls.
- No flat plate interiors → nowhere viable for players to build.
- Seams at wrap boundary if any stage isn't toroidal.
- Voronoi tie-breaking with floating-point distances → determinism issues.

**Mitigations:**
- Ensure meaningful variation in plate type and motion magnitude.
- Tune boundary falloff distance to be generous (wide ranges, not walls).
- Keep noise amplitude low in plate interiors, higher near boundaries.
- Use noise functions that tile in the wrap direction. Verify visually with debug PNGs.
- Deterministic Voronoi tie-breaking (e.g., lowest seed index wins).

**Output:** `HeightMap` (continuous elevation field + discrete levels) and `Plates` (retained for downstream climate/biome stages).

---

## 1.10 Stage 2: Climate

**Decision:** Two fields — temperature and moisture — computed from heightmap and latitude.

### Temperature

**Inputs:** Latitude (cosine curve, hotter at equator), elevation (lapse rate per height level), optionally proximity to water.

**Pole handling:** Map is pole-to-pole by default (top/bottom rows are arctic). Possible player option for mid-latitude maps.

**Output:** Temperature field as `Vec<f32>`.

### Moisture (via Advection Simulation)

**Decision:** Full advection-diffusion simulation, not the simpler ray-cast approximation.

**Rationale:** Richer moisture patterns (moisture pooling, complex multi-source effects). User's numerical background makes this implementation interesting rather than painful.

**Underlying physics:** 2D advection equation `∂M/∂t + ∇·(v·M) = S - P` discretized on hex grid, iterated to steady state. Pure advection (no diffusion term — natural diffusion from hex direction quantization is enough).

**Wind field:** Latitude-banded prevailing winds (easterly tropics, westerly mid-latitudes, easterly polar) plus low-frequency noise perturbation. Noise prevents visible horizontal stripes in output.

**Per-iteration update for each tile T (double-buffered):**
1. **Source:** Water tiles add evaporation scaled by temperature (warm water evaporates more)
2. **Advection:** Compute donor-cell transfers to downwind neighbors. For each of 6 neighbor directions, compute dot product with wind vector; positive dot products give transfer weights; move moisture proportionally.
3. **Precipitation:** Drop fraction of moisture as rain. Precipitation factor depends on:
   - Temperature (cold air holds less moisture)
   - Elevation change (orographic effect: large fraction precipitates when moisture moves uphill)
   - Base rate
4. Accumulate precipitation into output field P; subtract from working moisture M.

**Critical numerical constraints:**
- **CFL stability:** maximum fraction transferred from any tile per iteration ≤ 0.5 (safety margin under 1.0). Wind speed and iteration count are coupled.
- **Double buffering:** never update moisture in place; swap buffers each iteration.
- **Iteration count:** 100–300 iterations for 240×150 map, fixed count for determinism.
- **Floating-point determinism:** sum neighbor contributions in fixed order; avoid parallel reductions in inner loop unless using deterministic reduction.

**Orographic precipitation:** The most visually important effect. When moisture moves from a lower tile to a higher tile, drop 60–80% of transferred moisture as precipitation on the higher tile. Scales with elevation difference. This single rule produces realistic rain shadows automatically.

**Wrap handling:** East-west wrap is natural (moisture leaving east enters west). North-south doesn't wrap; moisture transferred off the top/bottom row is lost from the system.

**Output:** Precipitation field `Vec<f32>` indexed by tile, accumulated over all iterations.

**Risks:**
- Parameter tuning is sensitive; precip rate, evaporation rate, orographic factor all interact.
- Visible stripes in output if wind field is too uniform.
- Pole regions getting weird due to no-wrap boundary.
- Inland seas not acting as moisture sources if "source tile" check is ocean-specific.
- Lake feedback: lakes generated in hydrology stage should affect climate, but climate runs first. Small error.

**Mitigations:**
- **Debug visualization is non-negotiable.** Build PNG dump utility for temperature, moisture, and elevation before running any real generation work. Tune parameters by iterating on PNGs, not in-game.
- Wind direction noise overlay with small amplitude.
- Verify pole boundary handling explicitly.
- Apply moisture source rule to all water tiles, not just ocean.
- Lake-climate feedback ignored for v1; iterate if visibly broken.

**Skipped:** Ocean currents, seasonal variation, monsoon-specific patterns. High cost, marginal gameplay impact.

---

## 1.11 Stage 3: Hydrology

**Decision:** Flow accumulation with hybrid sink resolution (small sinks filled, large basins become lakes) plus post-hoc canyon carving.

### Flow Network

**Algorithm:**
1. For each land tile, determine flow direction = lowest neighbor (deterministic tie-break)
2. Identify sinks (tiles with no lower neighbor)
3. **Hybrid sink resolution (Planchon-Darboux + lake detection):**
   - For each sink, flood-fill upward to find lowest spill point
   - If basin area below threshold → fill (raise tiles to spill elevation, becomes flat drainage)
   - If above threshold → mark tiles below spill elevation as lake, route flow through spill point
4. **Flow accumulation** via topological sort by elevation (highest first):
   - For each tile in order, add own moisture to flow value
   - Add flow value to downhill neighbor
   - One pass, O(tiles)
5. **River identification:** Tiles where accumulated flow exceeds threshold become rivers. River size scales with flow value (creek / river / major river, or continuous for rendering).

### Lakes

- Generated as part of sink resolution above.
- Distinct from rivers (no visible flow direction).
- Part of flow network (water enters via rivers, exits via spill point).
- Should be moisture sources for climate — **but ignored as feedback for v1** (small error, simpler pipeline).

### Floodplains

**Decision:** Include in v1.

- Tile adjacent to high-flow river, with elevation close to river's, becomes floodplain.
- Treated as a feature (in tile composition sense), not terrain type.
- Agriculturally rich. Historically critical (Nile, Mesopotamia, Indus).

### Deltas

**Decision:** Skip for v1.

- Proper simulation is hard; cheap version (adjacent ocean tiles marked as delta feature) doesn't reproduce real shapes.
- May add later if river-gameplay design pillar matters more.

### Canyons (Post-hoc Carving)

**Decision:** Include. Visually iconic and mechanically meaningful (chokepoints, barriers, defensive terrain).

**Algorithm (runs after flow accumulation and sink resolution):**
1. For each river tile, compute flow × local slope
2. Where product exceeds threshold:
   - Mark tile as canyon feature
   - Lower elevation by 1–2 height levels
   - Mark adjacent tiles perpendicular to flow direction as canyon walls (cliffs)
3. One pass over the river network, cheap.

**Rationale:** Erosion feedback simulation (stream power law iterated) is the realistic approach but high cost. Post-hoc carving uses the same flow × slope condition that real erosion follows, so canyons form in geologically correct locations (high-flow rivers through steep terrain — Grand Canyon pattern) without iterative simulation.

**Risks:**
- Canyon walls produce height delta ≥ 2 transitions, which interacts with the still-unresolved cliff impassability question. **This decision now pushes the cliff question to "blocking" — must be resolved before hydrology can be fully implemented.**
- Canyon carving doesn't update flow network (canyon's new lower elevation could theoretically change drainage). For canyons specifically this is fine — they form where water already flows, so flow direction is stable.

**Mitigations:**
- Resolve the cliff-impassability question explicitly. Canyon walls being impassable is realistic and produces interesting chokepoint gameplay; recommend impassable for canyon walls at least, even if other delta-2 cliffs use different rules.
- Don't iterate flow after carving. Document this as a known approximation.

### Risks (Hydrology Overall)

- Pit-lake artifacts if basin threshold is wrong (small depressions becoming lakes everywhere).
- Rivers failing to reach sea if sink resolution incomplete.
- Rivers in deserts if threshold too low (Note: legitimate desert rivers like the Nile *should* exist when wet mountains drain through dry regions — the test is whether the pattern looks geographically correct).
- Sink resolution involves flood fills and elevation comparisons; tie-breaking and iteration order matter for determinism.

### Mitigations

- Tune basin-size threshold carefully; verify with debug PNGs (flow accumulation on log scale).
- Audit flow chains: every flow path should terminate at sea or in a designated lake.
- Deterministic tie-breaking throughout sink resolution.

**Output:** River network (with flow values), lake tiles, floodplain features, canyon features and walls, drainage data.

**Possibly modified:** Elevation field (sink filling may have raised some tiles; canyon carving lowered others).

---

## 1.12 Stage 4: Biome Assignment

**Decision:** Whittaker-style 2D lookup table from (temperature, moisture) to biome, with elevation/hydrology overrides.

**Biome set (land tiles only):**
- Tropical rainforest
- Temperate forest
- Boreal forest / taiga
- Grassland / savanna
- Desert
- Tundra

Ocean, coast, and lake are terrain types determined earlier (elevation + hydrology), not biomes — they don't get biome-assigned.

**Algorithm:**
1. For each land tile, look up biome from 2D table indexed by (temperature bin, moisture bin)
2. Apply overrides:
   - **Hydrology:** river tiles, lake-adjacent tiles, and floodplains override toward wetter/fertile variants (e.g., Nile-style fertile strip through desert biome)
   - **Elevation:** the "mountain" height level is already impassable terrain and doesn't need a biome; the tile *one below* may want alpine treatment (decide later as biome or feature)
3. Optional smoothing: single-tile biome islands (one tundra tile in a forest) can flip to surrounding biome to avoid visual noise. Tune carefully — don't oversmooth legitimate boundaries.

**Forest density as a feature, not a biome:** The biome is "temperate forest" (climate zone). Whether a specific tile has dense forest / light woodland / cleared is a *feature* placed by noise during a post-biome pass. This separates climate-zone identity from actual tree cover, allowing cleared tiles in forest biomes (logically and gameplay-wise sound).

**Lookup table format:** RON config file, designer-editable. Discretized bins of (temperature, moisture) → biome. Continuous input fields, discrete output classification.

**Rationale:** Whittaker classification is the ecological standard and produces realistic biome distributions automatically from the temperature/moisture fields already computed. Most of the complexity was in climate; biome is the readout.

**Risks:**
- Blocky biome boundaries if input fields are discretized too early.
- Holes in lookup table (unhandled climate combinations) → undefined biomes.
- Single-tile biome islands look like artifacts.

**Mitigations:**
- Keep temperature/moisture continuous until biome lookup.
- Validate lookup table covers full plausible range.
- Optional smoothing pass with conservative threshold.

**Output:** Biome per land tile; forest density feature per applicable tile.

---

## 1.13 Stage 5: Resource Placement

**Decision:** Geological resource placement using plate data and biome filtering. Luxuries cluster by plate ("mineral character"); strategics partially cluster, mostly biome-distributed. Order: resources placed *before* starting positions (see 1.14 for the player-fairness mechanism that doesn't require inverting this order).

**Resource categories:**
- **Strategic:** iron, horses, coal, oil (later-tech), uranium (very late)
- **Luxury:** gems, spices, silk, wine, gold
- **Bonus:** wheat, cattle, fish, deer, stone

Specific resource list is content/balance data, expected to evolve.

**Per-resource rules (RON config):**
- Eligible biomes/terrain
- Required features (e.g., horses need flat terrain, deer need forests)
- Optional latitude preferences (spices in tropics, fur in cold)
- Rarity (target % of eligible tiles)
- Cluster behavior (clumped vs scattered)
- Minimum spacing

**Plate mineral character:**
- Each plate gets a "mineral character" roll during tectonic stage (retained as plate data through resource placement)
- Luxury resources weight heavily toward plates matching their mineral character → continents have characteristic resources (this continent has the jade, that one has the gold)
- Strategic resources use lighter plate weighting — every continent has *some* iron, but specific regions have more
- Produces real-world-style geographic monopolies that drive trade and conflict

**Placement algorithm per resource:**
1. Filter to eligible tiles (biome + terrain + plate weighting)
2. For clustered resources: place seeds via Poisson disk sampling, expand each to 2–4 tile cluster
3. For scattered resources: Poisson disk sampling with minimum spacing
4. Target count from rarity × eligible tile count

**Rationale:** Geological placement supports the realism design pillar and creates real gameplay dynamics (monopolies → trade, war). Plate mineral character is a natural extension of data already generated. Clustering is realistic and gameplay-relevant.

**Risks:**
- Resources concentrating in geographically uneven ways (handled at starting-position stage, not here).
- Over/under-resourced maps due to rarity tuning.
- Strategic resource scarcity as balance lever — too common = no constraint; too rare = stuck players.

**Mitigations:**
- Tune rarity conservatively, validate with debug visualization across many seeds.
- Accept geographic variation; rely on starting position selection to handle player fairness.
- No corrective placement at this stage (preserves geographic coherence; corrections happen at starting-position audit if needed).

**Resource visibility (gameplay concern, not generation):** Some resources (oil, uranium) may be hidden until appropriate tech researched. Generation places them; runtime handles visibility.

**Output:** Resource per applicable tile.

---

## 1.14 Stage 6: Starting Position Selection

**Decision:** Resource-aware viability scoring with **hard adequacy floor** for every selected start. Optional surgical audit for edge cases.

**The reroll problem this solves:** Players in conventional 4X games spend 10+ minutes regenerating maps for "good starts" because early-game RNG dominates skill. In multiplayer this corrupts competitive integrity. Hard adequacy floor eliminates the reroll behavior — every start meets the floor, so there's nothing to chase.

**Adequacy requirements per starting position (local area, ~5–7 tile radius):**
- At least one fresh water source (river, lake, or coast)
- Minimum count of farmable tiles (grassland, plains, floodplain)
- Minimum count of production tiles (hills, forest)
- At least one strategic resource
- **At least one luxury resource (any luxury — not a specific one)**

**Note on luxury adequacy and the monopoly mechanic:** The adequacy requirement guarantees that every player has access to *some* luxury — wine, gold, spices, jade, etc. — but does not guarantee any specific luxury. The plate-clustered monopoly mechanic (1.13) is preserved: signature luxuries still cluster geographically, and trade/conflict dynamics still emerge from players wanting luxuries they don't have. The adequacy floor only prevents the worst case of a player starting with *no* luxury access at all.

**Algorithm:**
1. Score every land tile for starting viability:
   - Local resource quality
   - Fresh water proximity
   - Defensibility
   - Penalties for impassable-only adjacency, extreme biomes
2. **Filter to tiles meeting all adequacy requirements** (hard filter)
3. Cluster high-scoring tiles; select one representative per cluster
4. Select N starting positions with spacing constraints (`min_spacing ≈ sqrt(map_area / player_count) × 0.6`)
5. **Audit step (rare):** if filter set is too small for N positions with required spacing, identify borderline-viable starts and add 1–2 resources to bring them to adequacy. Place added resources geologically plausibly (iron on hills, not horses in tundra).
6. Validate: score variance across selected starts within tolerance.

**Rationale:** Geographic coherence preserved on the body of the map (luxuries still cluster by plate, strategic resources still follow geology). Adequacy floor guarantees viable starts. Audit handles edge cases without flattening geographic character on typical maps.

**Why not place players first, then resources around them:**
- Inverts geography → gameplay causality (real civilizations arose *because of* resource geography, not despite it)
- Incompatible with plate-clustered luxuries (can't simultaneously cluster luxuries geologically AND guarantee everyone has them locally)
- The actual goal — adequacy floor, not equality — is solvable without inversion via filtered scoring

**Symmetric multiplayer maps:** Future consideration. Point-symmetric or mirrored maps for maximally fair competitive play. Less natural-looking but valuable for tournament/ranked contexts. Not v1.

**Risks:**
- Filter too strict → few viable positions → audit triggered frequently → flattens character.
- Filter too loose → adequacy floor doesn't hold → reroll problem returns.
- Player count too high for map size → no satisfactory placement possible.

**Mitigations:**
- Tune adequacy thresholds against typical maps; verify audit triggers rarely (target <10% of generations).
- If player count is infeasible for map size, fail honestly (regenerate or cap players) rather than corrupt geography.
- Show adequacy criteria in UI for player's starting position — builds trust, eliminates reroll psychology even on borderline maps.

**Output:** N starting positions for N players. Audit-added resources (if any) merged into final resource layer.

---

## 1.15 Player Control: Three-Layer Parameter Exposure

**Decision:** Three layers of generation parameter exposure, with achievement disable on the top layer.

### Basic Layer
**Purpose:** Player chooses *flavor*. Any value produces a playable, comprehensible map.
**Contents:** Map archetypes — pangaea / continents / archipelago / islands.

### Advanced Layer
**Purpose:** Player chooses *strategic environment*. Meaningful gameplay consequences, but all values produce playable maps.
**Target slider count: 6–8 maximum.** Discipline: if adding a 9th, remove something or move it to Full Tunability.

**Starter set (in priority order):**
1. Map size (small/medium/large/huge)
2. Land/water ratio (sea level target)
3. Climate (cold/temperate/hot — global temperature shift)
4. Age (young/standard/old — affects mountain prevalence, erosion smoothness)
5. Resource abundance (scarce/standard/abundant)
6. Number of players (interacts with map generation, especially spacing)
7. Map shape modifier (fragmentation level)
8. River density

### Full Tunability Layer
**Purpose:** Player gets *experimentation*. Extreme values may produce broken maps.
**Contents:** Every internal generation parameter (plate count, advection iterations, wind speed, orographic factor, evaporation rate, biome lookup table thresholds, etc.).

**Requirements for this layer:**
- Tooltips with brief mechanism explanations on each parameter
- Save/share named parameter configurations ("Map: Earthlike v3")
- Quick regenerate loop (parameters → seed → visual result, fast iteration)
- Full determinism — community sharing means non-determinism will be caught quickly

**Achievement policy:**
- Achievements work on any Basic or Advanced configuration (including extreme combinations)
- Full Tunability **disables achievements** when parameters have been modified from defaults
- Opening Full Tunability menu without modifications does *not* disable (don't punish curiosity)
- Implementation: blanket disable initially; refine to per-achievement classification if community feedback warrants

**Rationale:** Player choice is a key replayability driver. Three layers serve distinct audiences (casual flavor, strategic depth, experimental tinkering) without overwhelming any of them. Full Tunability also provides a beta-testing pool for algorithm behavior in edge cases — useful feedback for default tuning, with the caveat that this audience is not representative of typical players.

**Design implications acknowledged:**
- Three-layer parameter exposure is a commitment to serving simulationist players. Coherent with other design choices (advection climate, geological resource placement, supply networks). The game's identity leans simulationist rather than tightly-curated.
- UI design matters disproportionately for the "thinks they want depth but actually wants curation" middle audience — clear defaults, easy reset, preset support in Advanced are all important.

**Risks:**
- Advanced layer slider count creep over time → degrades into "Full Tunability with worse names."
- Full Tunability parameter interactions can produce broken maps in ways individual sliders can't predict.
- Full Tunability feedback is biased toward technical/experimental users; tuning defaults based on their input can drift away from typical-player experience.

**Mitigations:**
- Hard discipline on Advanced slider count. Each addition must justify itself against alternatives.
- Accept that Full Tunability can produce broken maps; make regeneration cheap and obvious. Tooltips and presets help users avoid the worst combinations.
- Treat Full Tunability feedback as algorithm/edge-case testing, not as default-tuning guidance.

---

## 1.16 Map Generation Design — Status

**Design phase complete.** Pipeline fully designed through all six stages (tectonics → elevation → climate → hydrology → biomes → resources → starting positions) with player control layers specified.

**Remaining work is implementation:**
- Shared `coords` module with wrap-aware math
- Debug visualization tooling (PNG dumps of intermediate fields)
- Determinism test scaffold
- Standalone CLI runner for fast iteration without Bevy
- Stage-by-stage implementation in pipeline order, visually validated before progression

**Tunable decisions that remain (resolvable during implementation):**
- Exact default map size (working assumption 240×150 ≈ 36K tiles)
- Specific resource list and rarity tuning
- Biome lookup table specifics
- Advanced layer final slider set
- Adequacy floor thresholds (farmable tile count, production tile count, etc.)
- Whether alpine-below-mountains is a biome or a feature
- Whether to include forest density as a feature pass

---

## 2. Cities

### 2.1 City as Entity
**Decision:** A city is its own entity with state (owner, population, queues, treasury, etc.) that references the tiles it occupies via `NetworkId`. Tiles hold an optional back-reference to their owning city for fast lookup; the city is the source of truth.

**Rationale:** Modeling cities as components on tiles duplicates state and creates synchronization problems. Entity + references is clean and matches the multi-tile city design.

### 2.2 Tile Ownership Granularity
**Decision (provisional):** At least two ownership tiers per city — **core tiles** (the city proper) and **developed tiles** (have buildings/districts on them). Possibly a third: **influenced tiles** (provide resources but undeveloped).

**Rationale:** Smaller tile scale creates room for nuance that wasn't possible with Civ-sized tiles. Distinct tiers matter for borders, combat, and visual identity.

**Future aspects:**
- Lock in whether the third "influenced" tier is needed or whether two tiers suffice.

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

### 2.4 City Founding and Growth
**Decision:** Founding is cheap. Growth is expensive.

Initial founding (via a settler-type unit) is inexpensive. A founded city grows painfully slowly on its own. Meaningful growth requires deliberate investment: trade routes feeding resources from the region, specialized growth-accelerating units, or empire-level mechanisms.

**Rationale:** Inverts the conventional founding cost model. Speculative city placement is acceptable *if unsupported cities are visibly useless*. Real investment goes into growing the cities that matter.

**Open question:** What does an unfed city actually do — die slowly, persist at minimum size, or grow at subsistence rate? Working assumption (not locked): **persist at minimum**. A city with no supply lines stays at population 1, produces almost nothing, costs almost nothing, holds territory. This rewards smart claiming without rewarding sprawl.

**Risks:**
- Cheap founding can recreate the patchwork sprawl problem if unsupported cities still have functional output.
- "Economic incentives to stimulate growth" is the weakest of the listed growth mechanisms — money is fungible and accumulates, reducing the choice to "do I have spare cash?"

**Mitigations:**
- Unsupported cities must be functionally inert (or close to it), not just slow.
- Replace or supplement pure-money growth incentives with mechanisms that force real tradeoffs: **migration** (population moves between cities, zero-sum within empire), **policies/edicts** (empire-level prioritization), **resource-specific stimulation** (food surplus drives growth, not gold).

### 2.5 Capital Mechanic
**Decision (provisional):** The capital is **not** intrinsically special. Its head start comes from starting conditions (initial settler placed, basic infrastructure, starting resources), not from ongoing capital-specific growth bonuses.

**Rationale:** A permanently-special capital locks the best city to the starting position regardless of geography, undermining the goal of meaningful city placement.

**Future aspects:**
- Decide whether "capital" is a mobile designation (e.g., largest/most-connected city gets administrative bonuses) or just descriptive.

### 2.6 The Four Dimensions of Growth
**Decision:** Growth is multidimensional, not a single "size" number. Four independent axes:

1. **Population** — driven by food supply and housing
2. **Footprint** — territorial claim, cheap and fast, mostly geographic
3. **Development** — buildings and infrastructure on claimed tiles; expensive and slow, driven by production and materials
4. **Specialization** — emerges from what gets developed

**Rationale:** A trading hub and a mining town aren't differentiated by "size" — they have different population, footprint, development intensity, and specialization profiles. Modeling these as independent axes matches the supply-network design and creates richer city identities.

**Future aspects:**
- **Interactions between the four dimensions are explicitly tabled for later.** This is a significant design surface (e.g., does footprint gate population? Does development gate specialization?) that needs dedicated discussion.

### 2.7 Production Model
**Status:** Not finalized.

The conventional "two queues (units, buildings)" model was challenged. The current direction leans away from it toward something closer to **resource allocation** (production capacity split by priority/policy) or Stellaris-style **job allocation** (jobs produce resources, buildings create job slots).

**Future aspects:**
- Pick a production/allocation model. This is gated on the four-dimensions interactions discussion.

### 2.8 Expansion vs Development
**Decision:** Claiming a tile (expansion) and building on a tile (development) are **separate actions** with separate costs.

**Rationale:** Conflating them (Civ 6) makes optimal play "always claim aggressively because claiming = function." Separating them lets territory be claimed speculatively/defensively while real investment is reserved for development. Also makes claimed-but-undeveloped territory worth fighting over.

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
**Decision:** AI city management and player-facing "automate this city" use the **same system**, exposed to the person as **policies/directives** (e.g., "prioritize defense," "focus economy," "prepare for war") rather than full auto-pilot.

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

### 3.11 Supply and Logistics

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

---

## 3. Cross-Cutting: Determinism and Multiplayer Safety

**Decision (carried forward from prior work):** All systems must be multiplayer-safe by design. Command layer, `PlayerId` on all actions, `NetworkId` on all entities. Map generation must be deterministic.

**Implications for everything above:**
- All RNG explicitly seeded and threaded.
- No iteration over non-deterministically-ordered collections in any system that affects game state.
- City growth, resource flow, AI decisions — all must be deterministic given identical inputs.

---

## 4. Open Topics — Roadmap

### Map generation design: COMPLETE

All six pipeline stages designed (tectonics → elevation → climate → hydrology → biomes → resources → starting positions) plus three-layer player control. Remaining decisions are tuning-level and resolvable during implementation. See section 1.16.

### Next phase: implementation infrastructure

Before implementing any generation stage, build the supporting infrastructure:

1. **Shared `coords` module** — wrap-aware coordinate math, used by both generation and runtime.
2. **Debug visualization tooling** — PNG dumps of intermediate fields (heightmap, plates, temperature, moisture, flow accumulation, biomes, resources, starting positions). Load-bearing for parameter tuning. Build before first real stage implementation.
3. **Determinism test scaffold** — verify same seed produces byte-identical `WorldData` across machines and across runs.
4. **Standalone CLI runner** — produce a map and dump PNGs without Bevy. Enables fast iteration (seconds, not game-start times).

After infrastructure, implement stages in pipeline order, visually validating each before moving on.

### City/gameplay topics tabled for after map generation

5. **Combat — substantially designed.** Tactical model, batch commands, standing orders, strategic-level focus, unit scale (company, 1UPT), archetypes (soft RPS), veterancy with hard-death heroes, rail stacking with splash damage, upgrade mechanics, combat resolution (hybrid deterministic + bounded RNG), and supply/logistics (three-state with tech-unlock-coupled categories, pillaging, infrastructure vulnerability gradient) all settled. Remaining: city capture, frontage/flanking, naval, air, hierarchical orders (v2+). See section 3.12.
6. **City management tedium / production directives** — apply the intent-level pattern (pillar 0.6) to city production. Continuous-production directives, doctrine settings, automatic delivery. Same UI metaphors as combat batch commands. See section 2.11.
7. **Interactions between the four growth dimensions** — does footprint gate population, does development gate specialization, etc.
8. **Production/allocation model** — queues vs allocation vs job slots. Interacts with 2.11.
9. **Specialization mechanics** — player choice, geography-forced, or emergent.
10. **Migration system** — if population can move between cities (recommended), the rules and infrastructure for it.
11. **Emergent vs declared regions** — final decision.
12. **Infrastructure mode set** — roads, rails, ports, and their tradeoffs.
13. **Unfed city behavior** — die, persist at minimum, or grow at subsistence. Working assumption: persist at minimum.
14. **Capital mechanic** — keep mobile/descriptive (current direction) or revisit.
15. **UI/legibility systems** — dashboards, alerts, overlays. Non-trivial dev budget. Especially important given pillars 0.2 (knowledge as progression) and 0.6 (intent-level specification).
16. **Tech-unlock traversal of impassable terrain** — bridges (edge-based), tunnels (tile-based), mountain passes. Data model decisions deferred until implementation.

### Implementation-level decisions deferred from map gen

16. Default map size (working: 240×150).
17. Specific resource list and rarity tuning.
18. Biome lookup table specifics.
19. Advanced layer final slider set.
20. Adequacy floor thresholds for starting position viability.
21. Alpine zone as biome or feature.
22. Forest density as separate feature pass.

---

## 5. Known Issues, Risks, and Acknowledged Tensions

Issues identified during design review that must be tracked, stress-tested, or accepted explicitly. Distinct from "open topics" (which are undesigned) — these are problems with the current design that need ongoing attention.

### 5.1 Late-Game Turn-Time Performance (Major Risk)

**Concern:** Many systems independently push toward heavy per-turn computation:
- Climate advection at generation (less critical — runs once)
- Supply network resolution with multi-resource flow
- Per-tile economic dynamics for cities
- Combat with stacked modifiers (terrain × height × veterancy × supply × support × tech × unit-vs-unit)
- AI thinking at force level across many forces (3.4)
- Standing orders re-evaluated every turn for every unit (3.3)
- Region emergence from infrastructure topology (2.9)
- Multi-tile city updates (2.1, 2.6)

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

**Items that block downstream work:**
- **Interactions between the four growth dimensions (2.6)** — city design has structural holes until this is decided. Does footprint gate population? Does development gate specialization? Can't fully implement city mechanics without answers.
- **Production/allocation model (2.7)** — section 2.11 (city management tedium) prescribes solutions that assume a production model. Can't be implemented without it.
- **Emergent vs declared regions (2.9)** — affects infrastructure design, UI design, and trade flow implementation.
- **Specialization mechanics (open topic)** — interacts with city design, AI city management, and the four-dimensions question.

**Required action:** When implementation begins for city systems, these decisions must be resolved first. They are not optional refinements.

**Items that can genuinely wait:**
- Capital mechanic (descriptive only is fine for v1)
- Tech-unlock traversal of impassable terrain (only matters when those techs exist)
- Migration system (additive feature, not foundational)
- Naval/air combat (large but independent systems)

### 5.4 Unit Balance and Combat Validation

**Concern:** Combat sections 3.5-3.11 design mechanisms in detail but never address the foundational question of *what a unit's combat strength is and how it scales across tech eras*. Several mechanisms depend on this:
- Hard-death heroes (3.7) assume heroes are meaningfully stronger than green units
- Upgrade-vs-replacement (3.9) assumes upgrade cost is calibrated to combat value
- Combat resolution (3.10) assumes strength ratios are meaningful and tunable

**Status:** Cannot validate the combat design without rough balance numbers, even placeholder values. "Tuning" is treated as separate but some tuning affects whether mechanics work at all.

**Required action:** Before deep implementation of combat systems, establish a basic unit-strength model — even placeholder values — and verify that the designed mechanics produce sensible outcomes within that model. Iterate from there.

### 5.5 Standing Orders Default Behavior for Heroes (Minor Tension)

**Concern:** Pillar 0.6 (intent-level specification) drives standing orders toward automation. Hard-death heroes (3.7) require deep player attention to placement and risk. These are in mild tension — standing orders for heroes could lead to careless hero deaths.

**Resolution:** Hero units should have more conservative standing-order defaults than regular units. More alerts, more frequent player intervention requests, possibly an explicit "this is a hero, are you sure?" affordance when committing to risky orders. Standing orders on heroes are still allowed (the player retains control) but defaults should err on the side of preserving heroes.

**Status:** Minor design constraint to remember when standing orders are implemented in detail.

### 5.6 Narrative Texture (Future Pillar Candidate)

**Concern:** The design organically produces narrative gameplay moments — heroes emerging from battle veterans, Sherman-style pillaging raids, supply collapse during overextension, geographic monopolies driving wars, chokepoint battles at canyons and rivers. These emergent narratives are a real value of the design but are not currently stated as a pillar.

**Status:** Flagged for consideration as a future pillar (or value statement). If made explicit, future design decisions can deliberately preserve and amplify narrative texture. If not made explicit, future decisions might inadvertently flatten it.

**Worth considering when:** UI design, post-game summaries, or event/notification systems are designed. These are the places where emergent narrative is either surfaced (making it visible to players) or hidden (existing but unnoticed).

### 5.7 Player Count and Map Size Scaling

**Concern:** Advanced layer parameters include both map size and number of players (1.15). The relationship between them is unspecified. Edge cases — 12 players on a small map, 2 players on a huge map — need defined behavior.

**Required decisions:**
- What's the relationship between map size and supported player count? Hard limits, soft warnings, or unrestricted with consequences?
- When starting position adequacy can't be satisfied for N players, what happens? The doc says "fail honestly" but doesn't specify behavior — refuse to generate? Generate with warning? Reduce player count automatically?

**Status:** Minor but needs resolution before generation pipeline is exposed to the player. Tractable; just needs explicit design.

---

## 6. Document Hygiene Issues

### Section numbering

Section 3 appears twice — once for Combat (line 780) and once for Cross-Cutting (line 1190). The roadmap also has duplicate numbering (item 16 appears twice). These should be renumbered for clarity. (Cross-Cutting should probably be section 4, and the current section 4 becomes 5, etc. — but the renumbering is mechanical and can be done when convenient.)
