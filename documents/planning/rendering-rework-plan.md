# Rendering Rework Plan — Terrain Surface

**Status:** Planned
**Supersedes:** the geometry and texturing decisions in ADR-0019 and
ADR-0020 (see ADR-0026). Refines, but does not replace, ADR-0018
(chunked terrain rendering).
**Depends on:** Stage 3.x worldgen complete; `sim::StaticTerrain`
(ADR-0025) in place.
**Precedes:** feature, unit, and overlay render layers (Stage 4+).
**ADRs to write:** ADR-0026 (terrain renderer rework — drafted alongside
this plan).

---

## Goal

Replace the P1–P3 terrain renderer with a single continuous-surface
terrain system. The current renderer was scaffolding: it proved chunked
rendering, the texture-array pipeline, and the 3D scene, and taught the
shape of the problem. Its core geometry assumptions — per-tile hex fans,
vertical side quads, a raised-center "tent" for mountains, flat hills, a
per-vertex texture-layer index, and a fixed five-entry elevation table —
are all being discarded.

The replacement is one surface generator. Discrete elevation tiers remain
the simulation's truth; the renderer derives a smoothly-transitioned mesh
from those tiers and never reintroduces continuous height as state.
Hills and mountains stop being elevation tiers and become displacement
regimes layered on top of the tier surface. Biome boundaries blend across
a transition band instead of switching at tile edges.

The output is a continuous 3D strategy map — smooth slopes between
discrete levels, real hill and mountain shape, blended biome transitions,
and an animated-looking ocean — that is correct (watertight, level-count
agnostic), readable (tier boundaries stay legible), and within a fixed,
measured terrain frame budget on mid-tier hardware.

---

## Principles

These refine the project-wide implementation-plan principles for this
rework specifically.

**The mesh is downstream of discrete state.** Every vertex height,
biome weight, and surface normal is a pure function of discrete
`StaticTerrain` fields plus the world seed. The renderer holds no
height state of its own. This is the trust-tier discipline applied to
geometry: the surface can be rebuilt from scratch at any time and is
never a second source of truth. The single, well-defined tendril into
game state is edge classification (ramp vs. cliff), which is *core-owned*
because passability is mechanical.

**Frame-time is a first-class gate, judged on headroom.** Terrain is
the permanent floor under units, cities, improvements, combat effects,
and overlays, and the target user is not on high-end hardware. Terrain
must therefore consume a small, known fraction of the frame budget so
everything dynamic has the rest. The gate records vertex count, draw
count, and GPU frame-time as a fraction of budget on a defined mid-tier
target — not just "does it hit 60 on the dev machine."

**Prototype the linchpin first.** The three-tier corner is the geometry
decision every other piece conforms to. It is built and validated in
isolation, across all tier-delta and ramp/cliff combinations, before any
surrounding machinery is built. Building the easy two-tile edges first
risks redoing them to fit whatever the corner demands.

**Level-count agnostic by construction.** The elevation system must
produce correct, good-looking geometry for any `land_level_count`. Tier
height is computed from `mountain_level`, never from a hand-tuned fixed
table. A map with two land levels and a map with seven differ only in
total height, not in how a transition looks.

**Build for re-bake, do not build the producer.** Terrain mutation
(canals, tunnels) is a future gameplay feature with its own design. This
rework reserves the contract — a localized re-bake path and a
`TerrainChanged` event the renderer consumes — without building any
command that emits it.

---

## Scope

In scope:

- Teardown of the P1/P2/P3 terrain mesh, material, shader, and lookup.
- A new continuous terrain surface: solid hex cores at discrete tier
  heights, edge bridges, and three-tier corner stitches, watertight
  across chunk seams.
- Tier-height as a function of `mountain_level`.
- Hill and mountain geometry as displacement regimes on the core.
- Baked per-vertex normals from displaced geometry.
- A new terrain material: per-vertex three-biome blend weights and
  slope-gated triplanar projection, with a new WGSL shader.
- An ocean water shader: scrolling normal maps, depth-based color ramp,
  fresnel, and coastline-distance foam, fed by a baked coastline
  signed-distance field.
- `sim::edge_class` as core state, consumed by the renderer.
- A reserved `TerrainChanged` event and a pure, re-invocable chunk
  rebuild function.
- A terrain render layer as its own Bevy plugin, with clean boundaries
  so feature/unit/overlay layers slot in later.
- Perf gate; LOD-readiness (parameterized subdivision density) without
  building LOD itself.
- ADR-0026; `architecture.md` update.

Out of scope:

- The feature, unit, and overlay render layers themselves (only their
  plugin boundaries are established).
- Any command that mutates terrain; canal/tunnel gameplay.
- LOD selection logic (the door is left open; the room is not built).
- Shadows (the directional light remains; the toggle remains off).
- Authored terrain art beyond what already exists; the displacement and
  blend systems work with current textures and are art-replaceable.

---

## Decisions resolved before planning

These were settled in design discussion and are recorded here so the
stages below can assume them.

**Smooth slopes, discrete levels preserved.** Levels remain discrete;
the *transitions between* levels are smoothed. This is a deliberate
aesthetic choice aligned with the project's realism/simulation lean.
It is more work than stepped plateaus and commits to the full
bridge-and-corner lattice rather than vertical side faces.

**Tier boundaries stay legible at corners.** Smoothing applies across
genuine slopes. At three-tier corners and at cliffs, the geometry
preserves which tier (and therefore which passability) a position
belongs to, rather than smearing three tiers into an ambiguous blend.
Elevation is mechanical; the visual must not mislead the player about
it.

**Per-tile texturing is dropped.** The texture array survives; the
"one layer index per vertex" usage does not. It is replaced by
per-vertex blend weights over up to three biomes — the natural maximum
at a hex junction.

**Edge classification lives in `sim`.** `edge_class(a, b)` is core
state, not a rendering helper. It is read by movement, range, and
pathfinding as well as the renderer, and it derives from the same
tier-delta and passability rules already encoded in `movement_cost`
and the design doc §1.4 "delta ≥ 2 is impassable" rule (ADR-0012).

**Rewrite, do not preserve a baseline.** The old terrain path is
deleted, not kept for A/B comparison: it shares no mesh topology with
the new system, so a side-by-side diff would be meaningless, and git
retains it. Generic scene scaffolding (camera, lighting, app-state
sequence, chunk-spawn skeleton, texture-array asset loading) is kept.

---

## The surface model

The surface is a lattice in which **corners are primary data and tiles
are a stitching rule**. The naive alternative — each tile samples its own
corner/edge/center heights and builds an independent mesh — produces
cracks and z-fighting, because a point shared by several tiles is
evaluated independently by each and the results disagree. Inverting the
model makes watertightness hold by construction.

The concrete topology (the standard hex-terrain structure) has three
geometry kinds:

- **Solid core.** Each hex keeps a flat inner region at its discrete tier
  height. The flat core is what keeps the level legible. Cores of hilly
  or mountain tiles are additionally subdivided and displaced (below).
- **Edge bridge.** Between two adjacent hexes, a quad bridges their
  facing edges. A bridge across a one-tier passable delta becomes a
  ramp; a bridge across an impassable or large delta becomes a cliff.
- **Corner stitch.** At each three-hex junction a triangle (or small fan)
  stitches the three bridges. This is the three-tier corner — the common
  case on a hex grid, not an edge case.

**Dependency order.** Corners decide (height, and feed edge
classification); edges classify (flat / slope / cliff, from their two
tiers and core-owned passability); cores and bridge interiors are derived
(core = the tile's tier height plus any displacement; bridge tessellation
positioned from the corner and edge decisions). Center < edge < corner in
how much each decides.

**Watertightness across chunk seams.** A corner on a chunk boundary is
shared by tiles in different chunks. Two reconciling facts keep chunks
independent (preserving ADR-0018's rebuild boundary) while staying
watertight:

1. Corner height is a *pure deterministic function* of the tiers meeting
   there plus the world seed. Two chunks evaluating the same seam corner
   agree because they compute the same function, not because they share
   memory. "Evaluated once" becomes "evaluated by an agreed pure
   function."
2. To avoid emitting the same bridge or corner twice, each shared piece
   has a single **deterministic owner**: an edge bridge is owned by one
   of its two tiles, a corner stitch by one of its three, by a fixed
   rule (e.g. lowest `Coord`). The non-owning tiles emit nothing for
   that piece.

**Tier height from `mountain_level`.** World-space height is a function,
not a table:

```rust
// ------------- Tier heights -------------

// Ocean floor sits below the waterline; coast sits at the waterline (0).
// Each land level adds a constant increment, so the visual slope between
// any two adjacent levels is identical regardless of land_level_count.
const COAST_HEIGHT: f32 = 0.0;
const OCEAN_DEPTH: f32 = -8.0;
const LAND_STEP: f32 = 4.0;

// height_level: 0 = ocean, 1 = coast, 2..mountain_level = land,
// mountain_level = mountain.
fn tier_height(height_level: u8, mountain_level: u8) -> f32 { /* ... */ }
```

Constant per-step (rather than dividing a fixed range by the level count)
is deliberate: more levels make a taller world, not flatter-looking
steps. Hills and mountains are *not* represented here — they are
displacement on top of the land/mountain tier height.

**Displacement regimes.** Hill and mountain shape is deterministic noise
displacement on the subdivided core, keyed by `(coord, world_seed)` so it
is stable across re-bakes and identical across machines:

- A flat land tile: flat core, no displacement.
- A hilly tile: low-amplitude rounded displacement — gentle mounds.
- A mountain tile: high-amplitude craggy displacement. Contiguous
  mountain cores share corners and the same noise field, so a run of
  mountain tiles reads as a continuous ridge automatically — ranges are
  an emergent property of the shared surface, not an explicit extraction
  step. An optional cheap refinement (deferred unless needed): shape
  amplitude by mountain-neighbor count so interior tiles rise and
  perimeter tiles taper, giving ranges flanks.

Amplitude must go to zero at tile edges shared with non-displaced
neighbors so hills and mountains do not pop up as a hard wall at the core
boundary; the bridge/corner stitching then carries the transition.

**Normals.** Up-normals (`[0,0,1]`) light a displaced surface flat — it
has shape but no shading to reveal it. Normals are computed from the
baked geometry: averaged across shared corners for smooth shading on
slopes and hills, face normals on cliffs for a hard lip. Cheap at bake
time, non-optional.

**Determinism note.** The mesh is view-derived and does not enter the
worldgen determinism hashes. But all displacement noise is seeded from
the world seed and tile coordinate, so a re-bake of any chunk is
byte-identical to its original bake and identical across machines. This
keeps the future multiplayer story clean even though geometry is
presentation.

---

## Render layer separation

Presentation is subdivided by **update cadence**, because cadence
determines how each layer is built and when it rebuilds. Each layer is
its own Bevy plugin owning its meshes, materials, and systems; all
consume core state and events, none mutate state.

- **Terrain (this rework).** Static, baked, chunked. Re-bakes rarely, on
  `TerrainChanged`. Its plugin's `build` is the manifest of the terrain
  layer.
- **Features** (forests, rivers, resources, improvements). Semi-static,
  instanced, rebuilt on gameplay events. Not built here; only its
  boundary is respected.
- **Units and cities.** Dynamic, per-frame transforms and animation.
- **Overlays** (selection, movement range, fog). Ephemeral, per-frame.

This rework builds only the terrain layer and *establishes* the plugin
boundaries — the terrain plugin must not reach into feature/unit/overlay
code and vice versa. The term "pipeline" is avoided for these; in Bevy
that word means render-pipeline specialization. These are render
**layers**.

The re-bake contract:

```rust
// Reserved now; no command emits this yet. The renderer subscribes and
// re-bakes affected chunks. When canals/tunnels arrive, the command
// processor (the only tier permitted to mutate state) emits it.
#[derive(Event)]
pub struct TerrainChanged {
    pub coords: Vec<Coord>,
}
```

A note recorded for the future: `StaticTerrain` is currently documented
as immutable after insertion. Terrain mutation retires that invariant —
terrain is authoritative state, so mutations will route through the
command processor like everything else. This is flagged, not built, here.

---

## Stages

Each stage is one PR-sized slice with an explicit gate. Every gate
includes "passes `./scripts/check.sh`" (fmt, clippy pedantic, tests,
rustdoc) in addition to what is listed. The corner prototype (R1) gates
the entire rest of the plan.

### R0 — Teardown and scaffolding

Delete the old terrain mesh, material, shader, and lookup. Keep the
camera, lighting, app-state sequence, chunk-spawn skeleton, and
texture-array asset loading. Add `sim::edge_class` and the reserved
`TerrainChanged` event. Stand up an empty terrain-layer plugin whose
`build` lists what later stages will add.

`edge_class` is the only `sim` change:

```rust
// ------------- Edge classification (core-owned) -------------

/// Classification of the transition across a shared edge between two
/// adjacent tiles, derived from their tier delta and passability.
pub(crate) enum EdgeClass {
    /// Same tier — flat meeting.
    Flat,
    /// Small passable delta — a traversable ramp.
    Slope,
    /// Large delta or impassable — a near-vertical cliff.
    Cliff,
}

/// Classifies the edge between two adjacent tiles.
///
/// Core-owned because passability is mechanical game state: the
/// simulation decides a transition is a cliff; the renderer does not
/// infer it from appearance. Consumed by movement, range, pathfinding,
/// and the terrain renderer.
pub(crate) fn edge_class(a: Coord, b: Coord, terrain: &StaticTerrain) -> EdgeClass { /* ... */ }
```

Its relationship to `movement_cost` is settled during R0: `edge_class`
expresses the same delta/impassability rule the cost derivation already
uses, so the two are defined in terms of one shared predicate rather than
two parallel implementations that can drift.

**Gate:** project builds and runs with no terrain drawn (empty scene with
camera/lighting/sky). `edge_class` has unit tests covering flat, slope,
and cliff across the delta and `IMPASSABLE` cases. `check.sh` passes.

### R1 — Three-tier corner prototype (linchpin)

Build the three-tier corner in isolation, in a dedicated prototype
harness (a small `maprender`-style executable or a fixed synthetic
patch), exercising **all** combinations: three equal tiers; one-step and
multi-step deltas; ramp and cliff bridges in every arrangement around the
corner. Validate watertightness against the surrounding two-tier edges
and confirm tier-boundary legibility (the resolved decision — a position's
tier is visually unambiguous).

No chunked terrain, no biomes, no displacement yet. This stage exists to
fix the corner geometry that everything else conforms to.

**Gate:** every corner combination renders watertight (no cracks, no
z-fighting) under camera motion. Cliff corners show a hard lip; slope
corners show a smooth transition; mixed corners resolve without ambiguity
about which tier a point belongs to. Visual sign-off recorded (screenshot
set across combinations). `check.sh` passes.

### R2 — Full lattice, chunked, level-count agnostic

Generalize the corner prototype into the full surface: solid cores at
`tier_height(level, mountain_level)`, edge bridges classified by
`edge_class`, corner stitches, all emitted under deterministic ownership
so chunk seams are watertight with no double geometry. Cores are flat
(displacement comes in R3). Use existing textures via a temporary flat
assignment (real blending comes in R4) so geometry can be judged on its
own.

Make the mesh builder a pure function of `(chunk region, terrain
snapshot, edge classifications)` and parameterize core subdivision
density (LOD-readiness; one conservative density shipped).

**Gate:** maps at `land_level_count` of 2, 3, 5, and 7 all render
watertight with identical-looking per-step slopes. No seams between
chunks. Cliffs appear where `edge_class` says cliff. Re-invoking the
builder for one chunk reproduces it byte-identically. `check.sh` passes.

### R3 — Displacement regimes and baked normals

Add deterministic noise displacement to subdivided cores: low amplitude
for hilly tiles, high amplitude for mountain tiles, zero at edges shared
with non-displaced neighbors. Confirm contiguous mountain tiles read as
continuous ranges. Compute per-vertex normals from the displaced
geometry — averaged on slopes and hills, face normals on cliffs.

**Gate:** hills show real rounded shape under the directional light (not
just a color change); mountain runs read as ranges, not isolated spikes;
displacement is seam-free at hill/flat and mountain/land edges; a re-bake
is byte-identical. Lighting visibly reveals hill and mountain form.
`check.sh` passes.

### R4 — Biome blending and slope-gated triplanar

Replace per-tile texture indexing with the new material. Each vertex
carries up to three biome layers and blend weights (a core vertex is
100% its biome; a corner carries its ≤3 junction biomes). The fragment
shader blends ≤3 array samples by interpolated weight. Apply triplanar
projection only where surface slope exceeds a threshold (steep
bridges, cliffs, mountain faces), planar UVs elsewhere, with a blend
band between — confining the expensive 3×-sample path to the small
fraction of fragments that need it.

**Gate:** biome transitions blend smoothly across the transition band
with no hard tile-edge switch; steep faces show no UV stretching; near-
flat surfaces use the cheap planar path. Fragment cost of the triplanar
path measured and confined to steep fragments. `check.sh` passes.

### R5 — Ocean shader (independent; may be pulled earlier)

The single water plane gains an animated-looking surface entirely in the
fragment shader: two scrolling normal maps at different scales/directions
summed for moving specular glints; a depth-based color ramp (deep ocean
darker/bluer, coast lighter); a fresnel term for grazing-angle
brightening; and a foam band keyed off distance to coast. Depth and foam
are fed by a coastline signed-distance field baked once at world setup
from the discrete ocean/coast/land mask — deterministic and view-derived,
the same trust-tier category as the lattice. No vertex displacement; no
wave simulation.

This stage depends only on the water plane and the discrete water mask,
not on the lattice, so it can be scheduled earlier if convenient.

**Gate:** ocean reads as moving water at the tilted camera (glints
animate, coast is visibly shallower, foam hugs coastlines). Cost is
fragment-only; the SDF bake is build-time. `check.sh` passes.

### R6 — Performance gate, LOD-readiness, ADR, architecture

Measure against the fixed terrain budget on the defined mid-tier target.
Record vertex count, draw count, peak memory, GPU frame-time as a
fraction of budget, average and minimum FPS during a full-map pan and an
orbit, and any Bevy startup validation warnings. Confirm the mesh builder
remains parameterized for a future LOD drop-in but ship a single density.
Finalize ADR-0026 and update `architecture.md` to record: terrain surface
is view-derived from discrete tiers; edge passability classification is
core-owned; render layers are cadence-separated plugins.

**Gate:** terrain frame-time within budget on the mid-tier target with
headroom for dynamic layers; numbers recorded in the ADR's gate table;
`architecture.md` reflects the new structure; `check.sh` passes.

---

## Risks

**The three-tier corner dominates the schedule.** It is deliberately
first (R1) and gates everything. If its geometry proves harder than
expected, the rest of the plan waits — which is correct, because the rest
conforms to it.

**Triplanar cost on weak GPUs.** Mitigated by slope-gating (R4): the 3×
path is confined to steep fragments. The perf gate (R6) measures it
explicitly rather than assuming the gate is the dev machine.

**Texturing is the largest hidden chunk.** Moving off per-tile indexing
touches the mesh attributes, the material, and the WGSL shader together.
It is isolated in its own stage (R4) after the geometry is settled, so
geometry bugs and texturing bugs are not debugged simultaneously.

**`StaticTerrain` immutability vs. future canals.** Not a risk to this
rework (no producer is built), but flagged so the future feature routes
terrain mutation through the command processor rather than quietly
violating the invariant.

---

## Open decisions (resolve when the stage arrives)

- **Mountain amplitude shaping by neighbor count** (R3) — ship uniform
  high-amplitude displacement first; add neighbor-count flank shaping
  only if uniform ranges read poorly.
- **Exact slope threshold for triplanar gating** (R4) — tune against
  measured fragment cost and visible stretching.
- **Foam band width and normal-map scroll speeds** (R5) — tune visually.
- **Subdivision density** (R2/R6) — one conservative value shipped; LOD
  densities chosen only if the perf gate shows vertex count is the
  bottleneck.
