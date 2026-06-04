# ADR-0026: Terrain Renderer Rework — Continuous Surface

**Status:** Accepted
**Date:** 2026-06-01
**Deciders:** Kristoffer

## Context

The P1–P3 terrain renderer (ADR-0018, ADR-0019, ADR-0020) was built to
prove chunked rendering, the texture-array pipeline, and the 3D scene
pivot, and to learn the shape of the rendering problem. Now that the game
crate has its `sim`/`input`/`rendering` structure and a real
`StaticTerrain` model (ADR-0025), the terrain renderer is being reworked
from the ground up.

The P3 geometry has limitations that are not incremental fixes:

- Elevation transitions are vertical side quads (stepped), but the target
  aesthetic is smooth slopes between discrete levels.
- Mountains are a single raised center vertex per tile (a "tent"), which
  does not read as connected ranges.
- Hills carry no geometry — they are a darker texture on a flat top face.
- Tier heights are a fixed five-entry table (`ELEVATION_HEIGHTS`) that
  does not handle varying `land_level_count` and conflates the orthogonal
  `is_hilly` boolean with a height tier.
- Texturing assigns one texture-array layer index per vertex, which
  cannot express a corner shared between tiles of different biomes and
  stretches badly on steep faces.

These assumptions are all being discarded. The rework keeps the generic
scene scaffolding (camera, lighting, app-state sequence, chunk-spawn
skeleton, texture-array asset loading) and the chunked rendering model
(ADR-0018), and replaces the terrain mesh, material, shader, and lookup.

## Decision

Render terrain as a single continuous surface that is a pure view of
discrete elevation tiers.

**Surface model.** A corner-primary lattice: corners are evaluated by a
pure deterministic function of the tiers meeting there plus the world
seed; tiles are stitching rules. Geometry is solid hex cores at discrete
tier heights, edge bridges between adjacent tiles, and three-tier corner
stitches. Watertightness holds by construction. Chunk independence
(ADR-0018) is preserved by computing shared corners from an agreed pure
function and assigning each shared bridge/corner a single deterministic
owner so no geometry is emitted twice.

**Tier height from `mountain_level`.** World-space tier height is a
function of the discrete height level and the precomputed
`mountain_level`, with a constant per-level increment, rather than a
fixed table. This makes the renderer agnostic to `land_level_count`.

**Hills and mountains are displacement regimes, not tiers.** Hilly and
mountain tiles displace their subdivided cores with deterministic noise
keyed by `(coord, world_seed)` — low amplitude for hills, high amplitude
for mountains — with amplitude going to zero at edges shared with
non-displaced neighbors. Contiguous mountain tiles share corners and the
noise field, so ranges emerge from the shared surface without explicit
ridge extraction.

**Normals are baked from displaced geometry** — averaged across shared
corners on slopes and hills, face normals on cliffs.

**Texturing is per-vertex biome blending with slope-gated triplanar.**
The texture array is retained; the per-vertex layer index is dropped.
Each vertex carries up to three biome layers and blend weights (the hex
junction maximum); the fragment shader blends them. Triplanar projection
is applied only where slope exceeds a threshold; planar UVs are used
elsewhere, confining the 3×-sample cost to steep fragments.

**Edge classification is core-owned.** `sim::edge_class(a, b)` returns
flat / slope / cliff from tier delta and passability. It is consumed by
movement, range, and pathfinding as well as the renderer, and is defined
in terms of the same predicate as `movement_cost` so the two cannot
drift. This is the single tendril from the presentation surface into game
state.

**Ocean is a fragment-shader effect.** A single water plane uses two
scrolling normal maps, a depth-based color ramp, fresnel, and
coast-distance foam, fed by a coastline signed-distance field baked once
at world setup from the discrete water mask. No vertex waves; no
simulation.

**Render layers are cadence-separated plugins.** Terrain is its own Bevy
plugin (static, re-baked on a reserved `TerrainChanged` event), with
clean boundaries so the later feature, unit, and overlay layers slot in
without reaching into terrain.

**Frame-time is a first-class gate.** Terrain must fit a fixed fraction
of the frame budget on a defined mid-tier target, leaving headroom for
dynamic layers. The gate records vertex count, draw count, memory, and
GPU frame-time as a fraction of budget — not just FPS on the dev machine.

This decision supersedes the geometry and texturing decisions in
ADR-0019 (per-vertex texture-layer indexing) and ADR-0020 (P3.2 hex-prism
geometry: per-tile fan, vertical side faces, raised-center mountain, flat
hills, fixed elevation table). It refines ADR-0018 (chunked rendering),
which remains in force.

## Consequences

**Positive:**

- Smooth slopes with preserved discrete-level legibility match the
  realism/simulation direction.
- Hills and mountains gain real shape; ranges emerge from the shared
  surface for free.
- The renderer is `land_level_count`-agnostic, fixing a defect in the
  fixed-table approach.
- Biome transitions blend instead of switching at tile edges.
- The mesh is fully view-derived; a chunk can be re-baked byte-
  identically, keeping multiplayer determinism clean even though geometry
  is presentation.
- Edge classification is shared by gameplay and rendering rather than
  duplicated.
- Render layers are cleanly separated by cadence for future work.

**Negative:**

- The three-tier corner is genuinely fiddly and gates the rest of the
  work; it must be prototyped first.
- A new material and WGSL shader (biome blending + triplanar) replace the
  P2/P3 shader; triplanar cost must be measured on weak hardware.
- The continuous surface has a higher vertex count than the P3 prism
  mesh; this is build-time and static, but the perf gate must confirm
  headroom rather than assume it.
- `StaticTerrain`'s "immutable after insertion" contract will be retired
  when terrain mutation (canals/tunnels) arrives; flagged now, routed
  through the command processor then.

## Alternatives considered

- **Stepped plateaus (vertical side faces, as in P3.2).** Watertight
  without corner interpolation and very readable, but rejected because
  the target aesthetic is smooth, realistic terrain. This was a
  deliberate aesthetic choice, accepting the additional complexity of the
  full lattice.
- **Explicit mountain ridge-spine extraction.** Skeletonizing connected
  mountain components into meshed polylines gives the best "real range"
  look but is the most code and has the renderer re-deriving structure
  the discrete mask already encodes — over-engineering for a view layer.
  Rejected in favor of emergent ranges from the shared displaced surface;
  revisit only if the procedural result reads poorly.
- **Instanced mountain models (Civ-6 style).** Art-dependent and the
  seam-matching to look continuous across tiles is fiddly; conflicts with
  deferring art until gameplay is solid. Deferred.
- **Keeping per-tile texture indexing.** Cannot express shared corners
  between differing biomes (the point of the lattice) and stretches on
  steep faces. Rejected.
- **A painted splatmap for biome blending.** Redundant: blend boundaries
  are defined by tile topology, not free-hand painting, so per-vertex
  weights derived from the lattice are simpler and sufficient.
- **Gerstner / vertex-displaced ocean waves.** Most cost for least
  benefit at a tilted strategy camera, where wave silhouettes are rarely
  seen; needs a tessellated plane. Rejected for the fragment-shader
  approach; revisit only for a future near-water cinematic camera.
- **Building the terrain-mutation command path now.** Premature — canals
  and tunnels are an undesigned gameplay feature. Only the re-bake
  contract (`TerrainChanged`, a pure re-invocable builder) is reserved.
- **Keeping the old renderer for A/B comparison.** Rejected: it shares no
  mesh topology with the new system, so the diff would be meaningless,
  and git retains it.

## References

- ADR-0018 (chunked terrain rendering — refined, still in force)
- ADR-0019 (texture pipeline — texturing decision superseded)
- ADR-0020 (3D scene pivot — P3.2 geometry decisions superseded)
- ADR-0012 (cliff impassability; design doc §1.4 delta ≥ 2 rule)
- ADR-0025 (worldgen→game anti-corruption layer; `StaticTerrain`)
- `documents/planning/rendering-rework-plan.md`
- `documents/planning/elevation-transition-methodology.md`
