# Map Rendering Plan

**Status:** P1 complete, P2 complete, P3.1-P3.4 implemented  
**Depends on:** Stage 3.5 (resource placement) complete  
**Precedes:** Stage 4 game scaffolding (cities, units, dynamic systems)  
**ADRs written:** ADR-0018 (chunk rendering approach), ADR-0019 (texture pipeline)  
**ADRs to write:** ADR-0020 (3D scene pivot), ADR-0021 (orbit camera)

---

## Goal

Produce a mostly-complete map rendering layer before building any
dynamic game systems. Cities, units, and UI all sit on top of the
terrain; building them on an unproven rendering foundation creates
rework. This plan establishes that foundation in three stages:

- **Stage P1** — Performance gate: chunked vertex-color rendering, 60 FPS
  validated. **Complete.**
- **Stage P2** — Texture pipeline: texture array material, placeholder
  flat-color textures, same performance target. **Complete.**
- **Stage P3** — Full 3D scene: perspective camera with interactive
  orbit controls, hex prism terrain geometry with elevation tiers,
  directional lighting, Kenney forest models. Development-quality
  visuals sufficient to build cities and units against.

The output of P3 is a 3D strategy map — readable terrain elevation,
interactive camera, instanced forest models — good enough to build
Stage 4 against without being misled by the visuals.

---

## Architecture overview

The rendering stack has four layers. Layers are independent; each can be
updated without touching the others.

```
Layer 0: Terrain chunks      — 3D hex prism meshes baked from WorldData;
                               one entity per 16×16 chunk (~150 entities
                               for default map size). Static after load.
Layer 1: Feature overlays    — rivers (texture overlay on terrain),
                               forests (instanced 3D models),
                               resources (instanced icons or small models)
Layer 2: Dynamic overlays    — selection highlights, movement range,
                               fog of war; per-entity, small N at any
                               given moment (Stage 4+)
Layer 3: Units and cities    — 3D models loaded from .glb files (Stage 4+)
```

This plan covers Layers 0 and 1 through P3. Layers 2 and 3 are out of
scope.

The terrain layer is permanently static after world load. Chunk
invalidation (for future terrain-modifying features like channel
digging) is handled by rebuilding the affected chunk's mesh — no
architectural change required.

---

## Completed stages

### Stage P1: Performance gate (complete)

Chunked vertex-color rendering. 150 chunk entities for 240×150 map.
Orthographic 2D camera with WASD pan and scroll zoom. FPS overlay.
Gate passed. ADR-0018 written.

Key files: `rendering/hex_mesh.rs`, `rendering/chunk_spawner.rs`,
`rendering/camera.rs`, `rendering/terrain_color.rs`.

### Stage P2: Texture pipeline (complete)

Replaced vertex colors with a texture array material. Custom WGSL
shader samples a `texture_2d_array` by per-vertex layer index.
Programmatically generated 64×64 placeholder PNGs for all terrain
types. `AppState` loading sequence: assets load before chunks spawn.
Gate passed. ADR-0019 written.

Key files: `rendering/terrain_material.rs`, `rendering/terrain_lookup.rs`,
`app_state.rs`, `assets/textures/terrain/`.

---

## Stage P3: Full 3D scene

**Goal:** Pivot from 2D flat rendering to a full 3D scene with
perspective camera, elevation geometry, directional lighting, and
instanced forest models from Kenney's nature pack. This is the final
rendering foundation that Stage 4 builds on.

**Rationale for 3D pivot:** The map's elevation system — ocean, coast,
flat land, hills, mountains — is the primary visual differentiator
between terrain types. A flat top-down 2D view cannot convey elevation
without purely color-based cues, which are ambiguous and unreadable at
small tile size. A tilted 3D camera with actual height geometry makes
elevation immediately legible. Committing to 3D now, before cities and
units exist, avoids a later forced migration. The long-term direction
(Stellaris-inspired 3D RTS) makes this the correct permanent
architecture. Recorded in ADR-0020.

**Scope:**
- Perspective camera with interactive orbit controls (pan, zoom, tilt,
  rotate)
- Hex prism terrain geometry with discrete elevation tiers and side faces
- Peaked mountain tile geometry
- Texture array extended for top and side faces
- Bevy asset processor configured (PNG → BC7 at build time)
- Directional light (sun), shadows off, documented toggle to enable
- Kenney nature pack forest models as instanced 3D objects
- `ClearColor` sky color
- Performance measurement against P2 baseline

**Assets available for P3:**

*Terrain top-face textures (256×256 PNG, authored in Krita):*
`ocean.png`, `coast.png`, `grassland.png`, `tropical.png`,
`temperate.png`, `boreal.png`, `desert.png`, `tundra.png`,
`mountain.png` — all nine base terrain types covered.

*Foliage models (Kenney CC0, `.glb`):*
- Tropical: `tree_tropical_A.glb`, `tree_tropical_B.glb`
- Temperate: `tree_temperate_A.glb`, `tree_temperate_B.glb`, `tree_temperate_C.glb`
- Boreal: `tree_boreal_A.glb`, `tree_boreal_B.glb`

*Resource models (Kenney CC0, `.glb`):*
- Strategic: `iron_A.glb`, `horses_A.glb`
- Luxury: `gold.glb`, `wine.glb`, `gems.glb`, `spices.glb`
- Bonus: `wheat.glb`, `stone.glb`, `fish.glb`

Note: within each resource category, models currently share the same
mesh (all luxury models are identical, all bonus models are identical).
This is sufficient to prove the placement pipeline and per-resource
dispatch; distinct per-resource models are a later art pass.

*Still programmatically generated (no authored art yet):*
- Hilly variants of all land biome textures
- Side face textures (`side_land`, `side_hill`, `side_mountain`,
  `side_coast`)

**Out of scope for P3:**
- Shadows (light present; one constant to enable)
- River geometry as 3D (remains texture overlay on terrain top face)
- Road/feature geometry
- Units, cities, any dynamic entities
- Per-resource distinct models within a category (all luxury share a
  mesh; differentiation is a later art pass)

---

### P3.1 Scene pivot: 3D camera and lighting infrastructure

This step replaces the `Camera2d` and unlit material from P1/P2 with a
full 3D scene. All other rendering code is unchanged at this step —
the terrain mesh is still flat. This keeps the diff small and isolates
the camera/lighting change from the geometry change.

**Camera replacement.** Remove `Camera2d` and replace with a
`Camera3d` entity using a perspective projection. The initial position
is above the map center looking down at a shallow angle — approximately
60° tilt from vertical, 800 world units above the terrain. This gives
a first-person view of the isometric-style perspective that 4X games
use.

The `Camera2d` → `Camera3d` change also requires updating the mesh
rendering path. P2 uses `Mesh2d` and `MeshMaterial2d` (Bevy's 2D
render pipeline). P3 switches to `Mesh3d` and `MeshMaterial3d` with a
`StandardMaterial` configured for 3D. The chunk spawner changes in one
place; the mesh geometry is unchanged until P3.2.

**Lighting.** Add a `DirectionalLight` entity representing the sun:

```rust
/// Primary directional light. Angle chosen to cast readable shadows
/// from the north-west — the strategy game convention since Civ 5.
/// Shadows are disabled for P3; flip SHADOWS_ENABLED to test them.
const SHADOWS_ENABLED: bool = false;

DirectionalLight {
    illuminance: 10_000.0,   // lux; approx. overcast daylight
    shadows_enabled: SHADOWS_ENABLED,
    ..Default::default()
}
```

The light's `Transform` is rotated to simulate sun position from the
north-west at roughly 45° elevation. Adjust `illuminance` during
tuning — too bright washes out textures, too dim makes the scene
murky.

Also add an `AmbientLight` resource at low intensity (~0.15 relative
to the directional light). Without ambient, the shadow side of
elevation steps goes pure black, which looks wrong even without shadow
casting enabled.

**Sky color.**

```rust
app.insert_resource(ClearColor(Color::srgb(0.53, 0.70, 0.87)));
```

A horizon-blue sky color. This is a single line but makes the scene
feel grounded. The specific value can be tuned; the important property
is that it reads as sky against the terrain colors.

**Gate for P3.1:** App renders the flat P2 terrain mesh in a 3D scene
with perspective camera. Lighting visibly affects the terrain (flat
tiles show uniform shading; the directional light angle is apparent).
FPS unchanged from P2 baseline.

---

### P3.2 Hex prism geometry

Replace the flat hex mesh (Z = 0 for all vertices) with hex prisms
that express elevation through actual height. This is the core
geometry change of P3.

**Elevation tiers.** Map `HeightLevel` values to world-space Z heights.
The exact values are tunable constants:

```rust
/// World-space height (Z) for each elevation tier.
/// Units are the same as HEX_SIZE (16 world units = one tile radius).
const ELEVATION_HEIGHTS: [f32; 5] = [
    -8.0,   // 0: ocean floor — below the waterline
     0.0,   // 1: coast — at waterline
     4.0,   // 2: flat land
    12.0,   // 3: hills
    24.0,   // 4: mountains (before peak offset)
];
```

The mountain height is the base of the prism; the peak geometry adds
additional height on top (see below). Ocean tiles sit below the
waterline — a flat water plane at Z = 0 (coast level) can be added
later as a separate mesh to cover ocean/coast tiles.

**Top face geometry.** The top face of each non-mountain tile is the
same flat hexagon from P1/P2, but translated to the tile's Z height.
`CORNER_ANGLES` and the UV layout are unchanged.

**Side face geometry.** Where adjacent tiles differ in elevation, the
lower tile's prism exposes a vertical face. Side faces are added during
chunk mesh construction by checking each tile's six neighbours:

For each tile `T` and each of its six neighbours `N`:
- If `height(N) < height(T)`: add a quad (two triangles) connecting
  `T`'s bottom edge (at `height(N)`) to `T`'s top edge at the shared
  boundary. This is `T`'s side face visible from `N`'s perspective.
- If `height(N) >= height(T)`: no side face needed on this edge (the
  neighbour is at the same height or taller; the face is hidden).

The side face quad uses a different texture index from the top face —
a rocky/earthy side texture that reads as the exposed cross-section of
the terrain. Side faces do not need UVs that tile continuously across
chunk boundaries; each face is textured independently.

**Neighbour lookup across chunk boundaries.** Side face generation
requires reading the height of neighbouring tiles, which may be in
an adjacent chunk. This is not a problem — chunk mesh construction
has access to the full `WorldData`, so neighbour lookups are a
`world.tectonics.height_level.get(neighbour_coord)` call regardless
of which chunk the neighbour belongs to.

**Mountain peaked top face.** For mountain tiles (`height_level ==
mountain_level`), the top face is not flat. The center vertex is raised
by a `MOUNTAIN_PEAK_OFFSET` above the six corner vertices:

```rust
const MOUNTAIN_PEAK_OFFSET: f32 = 16.0; // world units above prism top
```

The six triangles of the top face each become sloped — a tent shape
that reads immediately as a mountain from a tilted camera. The six
corner vertices remain at the prism's top Z; only the center vertex is
elevated. This is a one-line change in `push_hex_top` for mountain
tiles.

**Hill top face.** Hills use a flat top face at their elevation tier.
The visual differentiation from flat land comes entirely from the
height difference and side faces. No additional geometry is needed.

**Water plane.** A single large flat mesh at Z = 0 (coast level),
covering the full map extents, rendered with a semi-transparent blue
`StandardMaterial`. Ocean and coast tile tops sit at or below this
plane; land tiles protrude above it. This gives a clear waterline
without requiring per-tile water geometry. The water plane is one
entity, not chunked — it is always fully visible and has negligible
vertex count.

**Texture index table (full P3 set).** The P2 programmatic placeholders
are replaced by authored PNGs for all base terrain types. Side faces
and hilly variants are still programmatically generated. The complete
table:

| Index | Name | Source | Notes |
|---|---|---|---|
| 0 | `ocean` | Authored (Krita) | Deep water |
| 1 | `coast` | Authored (Krita) | Shallow water |
| 2 | `tropical` | Authored (Krita) | `TropicalRainforest` |
| 3 | `temperate` | Authored (Krita) | `TemperateForest` |
| 4 | `boreal` | Authored (Krita) | `BorealForest` |
| 5 | `grassland` | Authored (Krita) | `Grassland` |
| 6 | `desert` | Authored (Krita) | `Desert` |
| 7 | `tundra` | Authored (Krita) | `Tundra` |
| 8 | `mountain` | Authored (Krita) | Mountain peak top face |
| 9–16 | `*_hilly` variants | Programmatic | Darkened base per land biome |
| 17 | `side_land` | Programmatic | Side face, flat land tier |
| 18 | `side_hill` | Programmatic | Side face, hill tier |
| 19 | `side_mountain` | Programmatic | Side face, mountain tier |
| 20 | `side_coast` | Programmatic | Side face, coast → ocean |

Programmatic textures (hilly variants, side faces): dark desaturated
versions of the corresponding authored top-face color, with a vertical
gradient (darker at bottom) to suggest depth. Generated by
`tools/gen_placeholder_textures/`. The texture array requires uniform
resolution — all textures must be 256×256 to match the authored set.
The placeholder generator is updated to produce 256×256 output.

**Normal recalculation.** Top face normals remain `[0.0, 0.0, 1.0]`
(pointing up). Side face normals point outward from the prism — each
side face's normal is the horizontal direction from the tile center to
the shared edge midpoint. This ensures the directional light shades
side faces differently from top faces, reinforcing the height reading.

**Gate for P3.2:** Map visually shows clear elevation tiers. Ocean
tiles sit below the waterline. Hill tiles are visibly higher than flat
land. Mountain tiles have peaked tops. Side faces visible between
elevation transitions. FPS target maintained.

---

### P3.3 Interactive orbit camera

Replace the WASD pan / scroll zoom camera from P1/P2 with a full
orbit camera that supports pan, zoom, tilt, and rotation. This is the
camera architecture that remains for the full game.

**Orbit camera model.** The camera is defined by three parameters:

- **Pivot point** — the world-space point the camera looks at, on the
  terrain surface. Pan moves the pivot.
- **Distance** — how far the camera is from the pivot. Scroll changes
  distance.
- **Angles** — yaw (rotation around the vertical Z axis) and pitch
  (elevation above the horizon). Right-mouse-drag changes both.

The camera `Transform` is derived from these parameters each frame:

```
camera_position = pivot
    + distance * (
        cos(pitch) * cos(yaw),   // X component
        cos(pitch) * sin(yaw),   // Y component
        sin(pitch)               // Z component
      )
camera_look_at = pivot
```

This is the orbit / arc-ball pattern. The camera always looks at the
pivot; moving the pivot is the pan operation.

Store the orbit state as a `CameraOrbit` resource:

```rust
#[derive(Resource)]
pub struct CameraOrbit {
    pub pivot:    Vec3,
    pub distance: f32,
    pub yaw:      f32,   // radians; 0 = looking north
    pub pitch:    f32,   // radians; clamped to [MIN_PITCH, MAX_PITCH]
}
```

**Controls.**

| Input | Action |
|---|---|
| Middle-mouse drag | Pan (move pivot in the camera's ground plane) |
| Right-mouse drag | Orbit (change yaw and pitch) |
| Scroll wheel | Zoom (change distance) |
| WASD / arrow keys | Pan (keyboard alternative) |

**Pitch clamping.** Prevent the camera from going below the terrain or
flipping upside-down:

```rust
const MIN_PITCH: f32 = 0.20; // ~11° above horizon — low but not underground
const MAX_PITCH: f32 = 1.48; // ~85° — nearly top-down
```

The default pitch for a 4X strategy view is approximately 45°–60°
(π/4 to π/3 radians). Set the initial pitch to `PI / 3.5` (~51°) as
the default.

**Zoom clamping.**

```rust
const MIN_DISTANCE: f32 = 80.0;    // close zoom: individual tiles readable
const MAX_DISTANCE: f32 = 1_200.0; // far zoom: full map visible
```

**Pan speed scaling.** Pan speed scales with distance so the map
doesn't feel sluggish when zoomed out:

```rust
let pan_speed = orbit.distance * PAN_SPEED_FACTOR;
```

**Camera bounds.** In P3, pan is clamped to keep the pivot within
the map bounds (same as P1/P2). Wrap-aware camera (seamless
east-west pan for the cylindrical map) is deferred to Stage 4.

**Implementation note.** The community crate `bevy_panorbit_camera`
provides this behaviour out of the box. However, implementing it
from scratch is recommended here: the game will need custom behaviour
(terrain-following pivot height, wrap-aware pan, integration with
tile selection), and understanding the orbit math makes those
extensions straightforward. The implementation is approximately 120
lines in `rendering/camera.rs`. ADR-0021 records this decision.

**Gate for P3.3:** Camera orbits correctly. Pitch clamp prevents
underground or flipped views. Pan keeps pivot in map bounds. Zoom
range covers from individual tile inspection to full map overview.
No frame-time cost from camera updates.

---

### P3.4 Forest and resource model instancing

Place biome-matched tree models on forested tiles and resource models
on resource tiles. This exercises the full 3D model loading pipeline
and gives a realistic performance baseline for Layer 1 features.

**Forest assets.** Biome-matched variants from Kenney nature kit (CC0):

| Biome | Models |
|---|---|
| `TropicalRainforest` | `tree_tropical_A.glb`, `tree_tropical_B.glb` |
| `TemperateForest` | `tree_temperate_A.glb`, `tree_temperate_B.glb`, `tree_temperate_C.glb` |
| `BorealForest` | `tree_boreal_A.glb`, `tree_boreal_B.glb` |
| `Grassland`, `Desert`, `Tundra` | No tree models — these biomes have low or zero forest density in worldgen |

Forest placement uses the tile's `Biome` to select the correct variant
pool, then picks deterministically within the pool using the tile
coordinate as a seed.

**Resource assets.** One model per resource, grouped by category:

| Category | Models |
|---|---|
| Strategic | `iron_A.glb`, `horses_A.glb` |
| Luxury | `gold.glb`, `wine.glb`, `gems.glb`, `spices.glb` |
| Bonus | `wheat.glb`, `stone.glb`, `fish.glb` |

Within luxury and bonus categories, models currently share the same
mesh. The category grouping is therefore the functional signal: players
can distinguish strategic from luxury from bonus by model style. Per-
resource distinct models within a category are a later art pass,
requiring no code changes — swap the `.glb` file, the lookup table in
`resource_models.rs` is unchanged.

Resource models are placed at the tile's world-space center, raised
to terrain surface Z, with no jitter (resources should read as
deliberately placed, unlike organically scattered trees).

**Scale adjustment.** Kenney models are authored at 1 unit = 1 meter.
`HEX_SIZE` is 16 world units. Apply a uniform scale to bring models to
a readable size relative to the hex grid. Different base scales apply
per asset type — trees are taller than resource icons:

```rust
const TREE_SCALE: f32 = 8.0;     // tune against terrain geometry
const RESOURCE_SCALE: f32 = 4.0; // smaller than trees; icons, not landmarks
```

Exact values require visual tuning after P3.2 geometry is in place.

**Instancing.** Bevy batches entities sharing a mesh and material
handle into a single draw call automatically. Forest trees: grouped by
model variant (7 variants → ≤7 draw calls). Resource models: grouped
by model (9 models → ≤9 draw calls). Total Layer 1 draw calls: ≤16.

For a 240×150 map with ~30% forested tiles above threshold and ~2%
resource tiles: roughly 10,000–15,000 tree entities and ~700 resource
entities. Validate draw call count matches expectation during P3.5.

**Placement details.**

*Trees:*
- One tree per tile where `forest_density > FOREST_THRESHOLD`
- Variant selected deterministically from biome's pool using
  `hash(coord) % pool_size`
- Position jitter: ±20% of `HEX_SIZE` in X and Y, deterministic
- Rotation jitter: random yaw from `hash(coord)`, no pitch/roll

*Resources:*
- One model per tile where `ResourceData::resource` is `Some(_)`
- No position jitter — centered on tile
- No rotation — resource models face a consistent direction

```rust
const FOREST_THRESHOLD: f32 = 0.45;
```

**Asset loading.** All `.glb` files load through Bevy's `AssetServer`
during `AppState::LoadingAssets`. Forest and resource model handles
are tracked alongside terrain textures; the state transition to
`SpawningChunks` waits for all handles to report `LoadState::Loaded`.

**Gate for P3.4:** Biome-matched tree models visible on forested tiles.
Resource models visible on resource tiles, category-distinguishable by
model style. All models sit on the terrain surface (correct Z). Draw
call count ≤ 16 for the full Layer 1. FPS target maintained.

---

### P3.5 Performance measurement and gate

With the full P3 scene active (3D terrain geometry, directional light,
forest models), measure performance against the P1/P2 baseline.

**Pass criteria:**

- Minimum FPS ≥ 60 during continuous orbit camera movement (pan,
  rotate, zoom), measured over at least two full map traversals.
- No frame-time spikes exceeding 2× average during camera movement.
- Memory footprint < 750 MB (increased from P2's 500 MB budget to
  account for 3D geometry, lighting, and model assets).
- No Bevy validation warnings at startup.
- Forest and resource draw call count confirmed ≤ 16 (instancing working).

**Expected bottlenecks and mitigations:**

*Terrain geometry vertex count.* Hex prisms with side faces have
significantly more vertices than flat hexes. For 36K tiles with
average 2 side faces exposed per tile: ~36K × (7 top + 2 × 4 side) =
~540K vertices per chunk batch. This is well within GPU budget but
worth measuring. If vertex count becomes an issue, reduce side face
detail or merge coplanar side faces.

*Forest model count.* 10–15K tree entities with instanced rendering
should be negligible. If Bevy's automatic batching fails to group
instances (check with `WGPU_BACKEND=vulkan cargo run` and a frame
debugger), explicit `GpuInstancing` may be needed.

*Shadow map (if enabled).* Testing with `SHADOWS_ENABLED = true` at
P3.5 gives the shadow cost baseline even if shadows ship disabled.
Record both numbers.

Record minimum, average, maximum FPS; peak memory; draw call count
in ADR-0020.

---

### P3 file structure

Changes from P2. Files marked `(new)` are additions; `(modified)`
are extensions of existing files.

```
crates/game/src/
├── main.rs                          (unchanged)
├── app_state.rs                     (modified: add model loading to LoadingAssets)
└── rendering/
    ├── mod.rs                       (modified: register new systems, 3D light)
    ├── hex_mesh.rs                  (modified: prism geometry, side faces,
    │                                  mountain peak, neighbour height lookup)
    ├── terrain_lookup.rs            (modified: full texture index table with
    │                                  authored and programmatic entries)
    ├── terrain_material.rs          (unchanged: texture array shader still correct)
    ├── chunk_spawner.rs             (modified: Mesh3d/MeshMaterial3d, water plane)
    ├── camera.rs                    (replaced: orbit camera replaces 2D pan/zoom)
    ├── lighting.rs                  (new: DirectionalLight, AmbientLight, ClearColor)
    ├── forest.rs                    (new: forest model loading and placement)
    └── resource_models.rs           (new: resource model loading and placement)

assets/
├── worldgen/
│   └── textures/                    ← authored terrain textures (256×256 PNG)
│       ├── ocean.png
│       ├── coast.png
│       ├── grassland.png
│       ├── tropical.png
│       ├── temperate.png
│       ├── boreal.png
│       ├── desert.png
│       ├── tundra.png
│       └── mountain.png
├── textures/terrain/                ← programmatically generated (hilly + side faces)
│   ├── grassland_hilly.png
│   ├── tropical_hilly.png
│   ├── temperate_hilly.png
│   ├── boreal_hilly.png
│   ├── desert_hilly.png
│   ├── tundra_hilly.png
│   ├── mountain_hilly.png
│   ├── side_land.png
│   ├── side_hill.png
│   ├── side_mountain.png
│   └── side_coast.png
└── glb-assets/
    ├── foliage/
    │   ├── tree_tropical_A.glb
    │   ├── tree_tropical_B.glb
    │   ├── tree_temperate_A.glb
    │   ├── tree_temperate_B.glb
    │   ├── tree_temperate_C.glb
    │   ├── tree_boreal_A.glb
    │   └── tree_boreal_B.glb
    ├── strategic-resource/
    │   ├── iron_A.glb
    │   └── horses_A.glb
    ├── luxury-resource/
    │   ├── gold.glb
    │   ├── wine.glb
    │   ├── gems.glb
    │   └── spices.glb
    └── bonus-resource/
        ├── wheat.glb
        ├── stone.glb
        └── fish.glb

tools/gen_placeholder_textures/
└── src/main.rs   (modified: 256×256 output, hilly variants, side face textures)
```

---

### P3 gate: complete when

- App renders 240×150 map as 3D hex prisms with correct elevation tiers.
- Side faces visible between elevation transitions; mountains peaked.
- Water plane visible at coast/ocean level.
- Authored terrain textures (ocean, coast, biomes, mountain) visible
  and correctly assigned. Programmatic hilly and side face textures
  present and readable.
- Orbit camera: pan, zoom, tilt, rotate all functional. Pitch and zoom
  clamped correctly.
- Directional light and ambient light applied. Scene readable without
  shadows.
- Biome-matched forest models present on forested tiles, instancing
  confirmed.
- Resource models present on resource tiles; strategic, luxury, and
  bonus categories visually distinguishable.
- All P3.5 performance criteria met and recorded.
- `assets/glb-assets/SOURCES.md` documents Kenney CC0 license for all
  model assets.
- ADR-0020 written: 3D pivot rationale, geometry decisions, measured
  performance results.
- ADR-0021 written: orbit camera implementation decision.
- `documents/architecture/architecture.md` updated: 3D scene
  architecture described.
- `documents/planning/implementation-plan.md` updated: P3 marked
  complete, Stage 4 unblocked.

---

## Cross-cutting concerns

### Wrap-aware camera (deferred to Stage 4)

P3 clamps the orbit pivot to map bounds. Stage 4 implements seamless
cylindrical east-west wrap: a second copy of terrain chunk meshes
offset by one map width to the east, camera pivot teleport when
crossing the midpoint. This is a `chunk_spawner.rs` and `camera.rs`
change only — terrain geometry and material code are unaffected.

### Chunk invalidation

When a terrain-modifying event occurs (channel digging, deforestation),
the affected chunk's mesh is rebuilt by calling `build_chunk_mesh`
for that chunk and swapping its `Mesh3d` handle. The chunk entity
structure already supports handle replacement; no architectural change
is needed. The forest layer must also be updated for the affected
chunk's tiles.

### Coordinate system

Axial `q` is east-west (wrapping), `r` is north-south (bounded).
World-space X increases eastward, Y increases northward, Z increases
upward (Bevy's right-hand Y-up convention). The hex center formula:

```rust
let x = HEX_SIZE * (SQRT3 * q + SQRT3 / 2.0 * r);
let y = -(HEX_SIZE * 1.5 * r);  // negate: positive r is southward
let z = ELEVATION_HEIGHTS[height_level as usize];
```

The Y negation is unchanged from P1/P2. Z is now meaningful —
elevation is expressed as actual world-space height.

### Asset pipeline: PNG to BC7

Bevy's asset processor can be configured to compress PNG textures to
BC7 at build time, storing the compressed result in `imported_assets/`.
Configure this in `AssetPlugin` during P3 setup:

```rust
AssetPlugin {
    mode: AssetMode::Processed,
    ..Default::default()
}
```

Add a `.meta` file for each texture specifying BC7 compression. The
first build is slower (compression runs); subsequent builds use the
cached compressed assets. This does not change the texture array
binding or shader — BC7 is transparent to the sampler. ADR-0019
should be updated to note this as the production texture format path.

### ADRs to write

- **ADR-0020** (at P3.5 gate): 3D scene pivot. Records the rationale
  for moving from 2D flat rendering to a full 3D scene, geometry
  decisions (stepped elevation tiers, peaked mountains, side faces),
  lighting setup, and measured performance results with and without
  shadows.
- **ADR-0021** (at P3.3 gate): Orbit camera implementation. Records
  the decision to implement the orbit camera from scratch rather than
  using `bevy_panorbit_camera`, with rationale (custom terrain-following
  and wrap-aware extensions would require forking the crate anyway).
