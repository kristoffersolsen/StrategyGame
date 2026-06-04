# Strategy Game — World Generation Design

---

## 1. Map Foundation

### 1.1 Tiling
**Decision:** Hexagonal tiling, pointy-top orientation, axial coordinates.

**Rationale:** Hex is the genre standard for good reasons (uniform neighbor distance, better movement feel). Pointy-top matches the coordinate implementation already used by the `coords` crate. Axial storage with on-demand conversion to cube for distance/LOS math.

**Risks:**
- Documentation and rendering must stay aligned with the coordinate implementation; a split orientation would make visual validation misleading.

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

**Implementation note:** Stage 3.4 stores the table at
`assets/worldgen/biome_table.ron` and validates full grid coverage at
pipeline startup. `BiomeParams` owns the bin counts; the table must
match those counts exactly. Hydrology overrides live in the table as a
deterministic `BTreeMap<Biome, Biome>`. See ADR-0016.

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
- **Strategic:** copper (ancient-tier material), iron, horses, coal, oil
  (later-tech), uranium (very late)
- **Luxury:** gems, spices, silk, wine, gold, tin
- **Bonus:** wheat, cattle, fish, deer, stone

Specific resource list is content/balance data, expected to evolve. The
*economic role* of each resource (yields, material tiers, amenities, gates) is
defined in `resources.md`, not here — this stage owns placement only.

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

**Implementation status (Stage 3.6):** The minimal implementation (viability scoring + greedy selection with spacing) is complete. The audit/rebalancing pass (step 5 above) and the luxury/strategic resource adequacy requirements are **deliberately deferred**. The implemented floor uses a neighbourhood yield-proxy sum (biome food + production, freshwater bonus, hilly bonus) without the full adequacy checklist. The audit pass requires a design decision on acceptable injection rate and resource placement rules that are blocked on open economy design (§5.3). Deferral is deliberate: the hard floor prevents inadequate starts; the audit adds edge-case robustness that is not required for the first playable iteration.

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
