# Tectonics Stage — Parameter Reference

Parameters for Stage 3.1. Consumed by `TectonicsParams` in
`crates/worldgen/src/params/tectonics.rs`. Set them via a RON preset
file (`--params <file.ron>`) passed to `maptest` or `mapdump`.

Two sets of defaults exist:

- **Struct defaults** — the values in `TectonicsParams::defaults()`. These
  define the determinism reference output. Treat changes to them as
  generation-output changes.
- **Preset defaults** — the values in `tools/maptest/presets/default.ron`.
  These are the tuned working values used for actual map generation.

When the columns differ, prefer the preset value for real runs.

---

## Step 1 — Plate seed placement

### `target_plate_count`

| | |
|---|---|
| Type | `u8` |
| Struct default | `10` |
| Preset default | `17` |
| Range | `4`–`32` |
| Sweepable | Yes |

Target number of Voronoi plate seeds. The Poisson-disk sampler fills the
map until no more seeds fit, so the realised count can be off by one or
two from the target. The minimum-spacing radius is derived from the target
and map area so the plate count is roughly invariant to map size.

**Tuning:** Fewer plates → large continental blocks, wide open oceans,
few boundaries. More plates → fragmented geography, many narrow
boundaries, higher mountain density. The range 12–20 produces varied
but coherent-looking worlds on a 240×150 map. Below 8, one or two
plates dominate the whole world.

---

## Step 3 — Plate properties

### `motion_magnitude_min` / `motion_magnitude_max`

| | Min | Max |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `0.5` | `1.5` |
| Preset default | `0.5` | `1.5` |
| Constraint | `> 0.0` | `≥ min` |

Each plate's motion vector has a magnitude drawn uniformly from
`[min, max]`. The scale is dimensionless — only relative magnitudes
between adjacent plates affect boundary classification.

**Tuning:** A wide range (e.g. 0.5–1.5) produces varied boundary types
because adjacent plates often have meaningfully different speeds. A
narrow range (min ≈ max) makes most boundaries transform (plates move
at similar speeds, so their relative motion is nearly zero normal to the
boundary). Raising both uniformly has no effect; only the ratio matters.

### `mineral_weights`

| | |
|---|---|
| Type | `Vec<f32>` (length 5) |
| Struct default | `[4.0, 2.0, 1.0, 3.0, 4.0]` |
| Preset default | `[4.0, 2.0, 1.0, 3.0, 4.0]` |
| Order | Ferrous, Precious, Gems, Hydrocarbons, Industrial |

Relative sampling weights for the five `MineralCharacter` variants.
Values are normalised before sampling, so `[1, 1, 1, 1, 1]` and
`[4, 4, 4, 4, 4]` are equivalent. At least one weight must be `> 0.0`.

**Tuning:** Not sweepable as a scalar; set via `--params <file.ron>`.
The default makes Ferrous and Industrial common, Hydrocarbons moderately
common, Precious uncommon, and Gems rare. This drives resource distribution
in Stage 3.5; changing weights here shifts which resources are geologically
abundant, not how densely they are placed on the map.

---

## Step 5 — Boundary classification

### `convergence_threshold`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.3` |
| Preset default | `0.01` |
| Range | `0.0`–`∞` (finite) |
| Sweepable | Yes |

The approach velocity between two adjacent plates (dot product of their
relative motion with the boundary normal) must exceed this threshold in
absolute value to be classified as convergent or divergent. Below the
threshold, the boundary is classified as transform.

**Tuning:** Lowering toward `0.0` makes almost every boundary convergent
or divergent, producing dense mountain ranges and trenches with few
transform (flat) boundaries. Raising toward `motion_magnitude_max`
makes most boundaries transform. The preset value of `0.01` is very
permissive — nearly all boundaries become convergent or divergent —
producing mountainous worlds; raise to `0.3`–`0.5` for flatter terrain
with more plains.

---

## Steps 6–7 — Elevation field

### `continental_base_elevation` / `oceanic_base_elevation`

| | Continental | Oceanic |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `0.3` | `−0.3` |
| Preset default | `0.1` | `−0.3` |

Base elevation applied uniformly to all tiles on each plate type, before
noise and boundary effects are added. The gap between the two values is
the primary control on how clearly continental and oceanic plates separate.

**Tuning:** The absolute values matter less than the difference. A gap of
`0.6` (e.g., `+0.3` / `−0.3`) produces clean continental/oceanic
separation; a narrower gap blurs the distinction. The quantile pass in
step 8 enforces the target land fraction regardless of base elevation, so
changing these values mainly affects the distribution of the continuous
elevation field, not the final land area.

### `intra_plate_noise_amplitude` / `intra_plate_noise_frequency`

| | Amplitude | Frequency |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `0.05` | `2.0` |
| Preset default | `0.05` | `2.0` |
| Constraint | `≥ 0.0` | `> 0.0` |

Low-frequency noise added to the base plate elevation to give plate
interiors texture. At default amplitude this is subtle — it breaks up
the uniform-elevation look without creating strong topographic structure.

**Tuning:** Raising amplitude introduces visible hills and valleys within
plate interiors, making elevation less uniform. Raising frequency makes
the interior variation finer-grained. Keep amplitude well below the
continental/oceanic base gap or the continent-ocean boundary becomes
ambiguous.

### `boundary_falloff_sigma`

| | |
|---|---|
| Type | `f32` |
| Struct default | `8.0` |
| Preset default | `1.0` |
| Constraint | `> 0.0` |

Standard deviation of the Gaussian used to propagate each boundary tile's
elevation modifier into surrounding tiles, in tiles. Effects fall to ~1%
of peak at 3σ.

**Tuning:** This is the primary control on mountain range width. At `σ = 8`
the Gaussian spreads ~24 tiles in each direction — very broad ranges. At
`σ = 1.0` ranges are narrow and sharp. The preset value of `1.0` combined
with the multi-pass step-7 detail noise gives crisp, narrow ranges with
rougher edges. Raising sigma also raises the total elevation added to
boundary tiles, inflating how many tiles reach the mountain threshold —
compensate by lowering `mountain_elevation_boost` proportionally.

### `boundary_strength_noise_amplitude` / `boundary_strength_noise_frequency`

| | Amplitude | Frequency |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `0.0` | `3.0` |
| Preset default | `0.0` | `3.0` |
| Constraint | `0.0`–`1.0` | `> 0.0` |

Coherent noise applied once per tectonic boundary source tile before its
modifier is propagated. The noise is evaluated only at boundary sources,
so it behaves like variation along the boundary: high sections emit the
full uplift or depression, while low sections fade toward gaps.

**Tuning:** Raise amplitude to break continuous mountain walls and island
arcs into chains. Raise frequency for shorter, more frequent gaps; lower
frequency for long range-scale segments. At amplitude `0.0`, this path is
an exact no-op.

### `boundary_sigma_noise_amplitude` / `boundary_sigma_noise_frequency`

| | Amplitude | Frequency |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `0.0` | `3.0` |
| Preset default | `0.0` | `3.0` |
| Constraint | `0.0`–`1.0` | `> 0.0` |

Coherent noise applied once per tectonic boundary source tile to vary its
local Gaussian falloff width. This approximates orthogonal variation
without computing explicit boundary tangent/normal geometry.

**Tuning:** Raise amplitude to mix narrow sharp sections with broader
diffuse sections. Raise frequency for rapidly changing range width. Local
sigma is clamped internally so propagation radius stays bounded.

### `mountain_elevation_boost`

| | |
|---|---|
| Type | `f32` |
| Struct default | `1.2` |
| Preset default | `0.4` |

Elevation added at continental–continental convergent boundary tiles
(before Gaussian spread). These are the highest mountain ranges.

### `coastal_mountain_boost`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.8` |
| Preset default | `0.6` |

Elevation added at the continental side of a continental–oceanic
convergent boundary. Produces coastal ranges (Andes-like).

### `subduction_trench_penalty`

| | |
|---|---|
| Type | `f32` |
| Struct default | `−0.8` |
| Preset default | `−0.3` |

Elevation added (typically negative) at the oceanic side of a
continental–oceanic convergent boundary. Produces subduction trenches.

### `island_arc_boost`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.4` |
| Preset default | `1` |

Elevation added at oceanic–oceanic convergent boundaries. Produces island
arcs and volcanic archipelagos.

### `rift_valley_penalty`

| | |
|---|---|
| Type | `f32` |
| Struct default | `−0.4` |
| Preset default | `−1` |

Elevation added (negative) at continental–continental divergent
boundaries. Produces rift valleys.

### `mid_ocean_ridge_boost`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.3` |
| Preset default | `0.3` |

Elevation added at oceanic–oceanic divergent boundaries. Produces
mid-ocean ridges (subsea mountain chains).

**Tuning the boundary modifiers together:** The six boundary modifiers
interact through `boundary_falloff_sigma`. All Gaussians from boundary
tiles within σ of each other sum; at high sigma, a tile near two
different boundary types accumulates both modifiers. The key lever is
the ratio of the mountain boost to the trench penalty — too small and
mountains/trenches are indistinct; too large and the quantile pass
reassigns most tiles to mountain. The preset values produce visible but
not dominant mountain chains at `boundary_falloff_sigma = 1.0`.
Use `boundary_strength_noise_amplitude` to break those chains into gaps
before reaching for `mountain_pass_threshold`, because boundary noise
changes the continuous elevation field while mountain-pass carving works
after quantization.

### `detail_noise_amplitude` / `detail_noise_frequency` / `detail_noise_octaves`

| | Amplitude | Frequency | Octaves |
|---|---|---|---|
| Type | `f32` | `f32` | `u8` |
| Struct default | `0.08` | `4.0` | `3` |
| Preset default | `3.5` | `8.0` | `6` |
| Constraint | `≥ 0.0` | `> 0.0` | `1`–`8` |

Multi-octave fBm noise added uniformly over the full elevation field
after boundary effects. This is the primary source of local elevation
variation at sub-plate scale: it makes coastlines jagged, produces
small hills and valleys within flat interiors, and roughens the edges
of mountain ranges.

**Tuning:** At the preset values (`amplitude = 3.5`, `frequency = 8.0`,
`6 octaves`) this is the dominant fine-scale signal, contributing more
to individual tile elevations than the plate base or boundary modifiers.
Raising amplitude beyond ~4.0 begins to wash out the large-scale
continental structure. Raising frequency and octaves adds finer
detail but increases computation. The struct defaults (`0.08`, `4.0`,
`3`) are conservative — the boundary effects dominate at those settings.

---

## Step 8 — Sea level and height quantisation

### `land_level_count`

| | |
|---|---|
| Type | `u8` |
| Struct default | `3` |
| Preset default | `5` |
| Range | `2`–`16` |
| Sweepable | Yes (rounded) |

Number of discrete height levels assigned to land tiles, not counting the
fixed ocean, coast, and mountain levels. Total height levels = `land_level_count + 3`.

**Tuning:** More levels give finer elevation gradation on land. Biomes,
hydrology, and resources all use `height_level` to distinguish terrain;
more levels give those stages more to work with. The `level_proportions`
vector must have `land_level_count + 3` entries — changing this parameter
requires updating that vector simultaneously.

### `level_proportions`

| | |
|---|---|
| Type | `Vec<f32>` |
| Struct default | `[0.35, 0.05, 0.25, 0.20, 0.10, 0.05]` (at `land_level_count = 3`) |
| Preset default | `[0.35, 0.05, 0.15, 0.1, 0.15, 0.1, 0.05, 0.05]` (at `land_level_count = 5`) |
| Constraint | Length = `land_level_count + 3`, all ≥ 0, sum = 1.0 |

Target fraction of tiles in each height band. Order: `[ocean, coast,
land_0, …, land_N, mountain]`. The quantile pass computes cutoffs on
the continuous elevation field that hit these fractions exactly.

**Tuning:** Not sweepable as a scalar; set via `--params <file.ron>`.
The first entry controls water fraction (combined ocean + coast is
`proportions[0] + proportions[1]`). The mountain fraction is
`proportions[last]`. Because the mountain pass in step 10 demotes
some mountain tiles to high-land, set `proportions[last]` ~40% higher
than the desired final mountain fraction to compensate — see ADR-0005.
Example: to end up with ~5% mountains, set `proportions[last] = 0.07–0.08`.

---

## Step 9 — Hills classification

### `hill_influence_radius`

| | |
|---|---|
| Type | `u32` |
| Struct default | `8` |
| Preset default | `8` |
| Unit | Tiles (hex distance) |
| Sweepable | Yes (rounded) |

Hex-distance within which proximity to a tectonic boundary increases
the probability of a tile being classified as hilly.

**Tuning:** Larger radius → more tiles benefit from proximity-based
hilliness. At `8` tiles roughly half the map's land is within range
of some boundary on a typical world. Reducing to `4`–`5` restricts
hilliness to tiles immediately adjacent to ranges; raising to `12`+
makes hilliness nearly universal near continental margins.

### `hill_near_boundary_threshold` / `hill_interior_threshold`

| | Near | Interior |
|---|---|---|
| Type | `f32` | `f32` |
| Struct default | `−0.2` | `0.55` |
| Preset default | `−0.2` | `0.55` |
| Range | `[−1, 1]` practical | `[−1, 1]` practical |

The hill-noise value (OpenSimplex, approximately `[−1, 1]`) at each
tile is compared against a threshold interpolated between these two
values based on boundary proximity. Tiles above the threshold are
classified as hilly.

**Tuning:** Lower threshold → more hilly tiles. At `−0.2` for
near-boundary tiles, roughly 60% of boundary-adjacent tiles are hilly.
At `0.55` for interior tiles, roughly 22% are hilly. The gap between
the two thresholds controls how strongly hilliness concentrates near
plate boundaries. Setting both to the same value makes hilliness
spatially uniform (not recommended for visual quality).

### `hill_noise_frequency`

| | |
|---|---|
| Type | `f32` |
| Struct default | `5.0` |
| Preset default | `5.0` |
| Constraint | `> 0.0` |

Spatial frequency of the hilliness noise field. Higher values produce
smaller, more scattered hill patches; lower values produce larger
contiguous hilly regions.

---

## Step 10 — Mountain pass carving

### `mountain_pass_threshold`

| | |
|---|---|
| Type | `f32` |
| Struct default | `0.15` |
| Preset default | `0.15` |
| Disable | Set `> 1.0` |

OpenSimplex noise threshold for mountain tile demotion. Mountain tiles
whose noise value exceeds this threshold are demoted to the highest land
level, creating traversable passes through ranges.

**Tuning:** At `0.0`, ~50% of qualifying tiles are demoted (most passes
are very fragmented). At `0.5`, ~25% are demoted. At the preset value
`0.15`, ~35–40% of interior mountain tiles become passes, producing
3–5 gaps per range on a typical world. Set above `1.0` to disable
entirely and produce impassable walls. Combine with `level_proportions[last]`
adjustment (see that section above) to compensate for the post-demotion
mountain fraction drop.

### `mountain_pass_noise_frequency`

| | |
|---|---|
| Type | `f32` |
| Struct default | `3.0` |
| Preset default | `10.0` |
| Constraint | `> 0.0` |

Spatial frequency of the mountain pass noise field. Higher values produce
more frequent, shorter gaps; lower values produce fewer, wider passes.

**Tuning:** At `3.0`, passes are on the scale of 5–8 tiles wide. At
`10.0`, passes are narrower and more numerous. The preset value `10.0`
produces compact tactical chokepoints; `3.0` produces broad saddles
more suitable for river valleys.

### `mountain_pass_neighbor_aware`

| | |
|---|---|
| Type | `bool` |
| Default | `true` |
| Sweepable | No (use `--params`) |

When `true`, a mountain tile is only eligible for demotion if at least
`mountain_pass_min_mountain_neighbors` of its six neighbours are also
at mountain level. This restricts demotion to range interiors, so passes
cut *through* ranges rather than nibbling their edges.

When `false`, any mountain tile is eligible regardless of neighbours.
Appropriate when `boundary_falloff_sigma` is low and mountain tiles are
scattered rather than forming coherent bands — see the `test.ron` preset
for an example with `boundary_falloff_sigma = 2.0`.

### `mountain_pass_min_mountain_neighbors`

| | |
|---|---|
| Type | `u8` |
| Default | `3` |
| Range | `0`–`6` |
| Sweepable | Yes (rounded) |

Minimum mountain-level neighbours required for eligibility when
`mountain_pass_neighbor_aware` is `true`. At `3`, the tile must have
at least half its neighbours at mountain level — it is embedded in
the interior rather than on the edge. Raising to `4`–`5` restricts
demotion to deeply interior tiles, producing passes only in very
broad mountain ranges. Lowering to `1`–`2` relaxes the filter,
allowing passes to form closer to range edges.
