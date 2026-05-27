# Map Rendering Plan

**Status:** Planning complete — ready for implementation  
**Depends on:** Stage 3.5 (resource placement) complete  
**Precedes:** Stage 4 game scaffolding (cities, units, dynamic systems)  
**ADRs to write:** ADR-0018 (chunk rendering approach), ADR-0019 (texture pipeline)

---

## Goal

Produce a mostly-complete map rendering layer before building any
dynamic game systems. Cities, units, and UI all sit on top of the
terrain; building them on an unproven, placeholder rendering foundation
creates rework. This plan establishes that foundation in three stages:

- **Stage P1** — Performance gate: chunked vertex-color rendering, 60 FPS
  validated.
- **Stage P2** — Texture pipeline: texture array material, placeholder
  flat-color textures, same performance target.
- **Stage P3** — Terrain visuals: authored placeholder art for all
  terrain types and features, visual quality sufficient for development
  use. Not final art.

The output of P3 is a map that looks like a game in development — readable,
unambiguous, good enough to build cities and units against — without
committing to final art direction.

---

## Architecture overview

The rendering stack has four layers. Layers are independent; each can be
updated without touching the others.

```
Layer 0: Terrain chunks      — static hex meshes baked from WorldData;
                               one entity per 16×16 chunk (~150 entities
                               for default map size)
Layer 1: Feature overlays    — rivers, forests, roads; separate chunk
                               meshes or sparse entities depending on
                               density (planned in P3)
Layer 2: Dynamic overlays    — selection highlights, movement range,
                               fog of war; per-entity, small N at any
                               given moment (planned in Stage 4+)
Layer 3: Units and cities    — standard Bevy entities with sprites or
                               meshes (Stage 4+)
```

This plan covers Layers 0 and 1. Layers 2 and 3 are out of scope.

The terrain layer is permanently static after world load. Features like
rivers and forests are also generated at world load and do not change
during a game session (channels, deforestation, and similar terrain
modifications are a late-game design concern that will be handled by
rebuilding affected chunks, not by redesigning the layer architecture).

---

## Stage P1: Performance gate

**Goal:** Prove that the chunked mesh approach scales to the target map
size (240×150 ≈ 36K tiles) at 60 FPS before building anything on top
of it. This is the gate described in `implementation-plan.md §3.5`.

### P1.1 Hex mesh geometry

Implement `build_chunk_mesh` in `crates/game/src/rendering/hex_mesh.rs`.
This function takes a chunk origin `Coord`, the map `MapBounds`, and a
reference to `WorldData`, and returns a Bevy `Mesh`.

**Hex geometry.** A pointy-top hex with circumradius `r` has six
vertices. In axial coordinates `(q, r_coord)`, the world-space center of
a tile is:

```
x = hex_size * (√3 · q  +  √3/2 · r_coord)
y = hex_size * (3/2 · r_coord)
```

This is the 3D equivalent of the formula in
`crates/worldgen/src/debug_viz.rs::hex_center`, adapted for mesh vertex
positions rather than image pixel centers. The `debug_viz` code is a
reference; do not reuse it — it is headless-only and outputs `f64` pixel
coordinates.

Each hex triangulates into a triangle fan from its center: 6 triangles,
7 unique vertices (center + 6 corners), 18 indices. All vertices of a
given tile receive the same vertex color (flat shading, no interpolation
across the face).

**Mesh attributes required for P1:**

- `POSITION` — `Vec3`, Z = 0 for all vertices (flat map)
- `COLOR` — `Vec4` RGBA, one color per vertex (same for all 7 vertices
  of a tile)
- `NORMAL` — `Vec3`, `[0.0, 0.0, 1.0]` for all vertices (flat map
  facing the camera)

The `NORMAL` attribute is required by `StandardMaterial` even with
`unlit: true`; omitting it causes a Bevy validation warning.

**East-west wrap.** Tiles whose axial `q` coordinate places their
world-space center outside the map's pixel-space width are wrapped using
`MapBounds::normalize` before computing world position. Chunks that
straddle the east-west seam are handled correctly because each tile is
positioned independently — the mesh is contiguous within a chunk
regardless of seam crossing.

**Chunk size constant:**

```rust
/// Tile count per chunk side. A chunk covers CHUNK_SIZE × CHUNK_SIZE tiles.
///
/// 16 is the conventional default for strategy-game tile renderers:
/// small enough for good frustum-culling granularity, large enough
/// that draw call count (~150 for the default map) is negligible.
/// See ADR-0018.
pub const CHUNK_SIZE: u32 = 16;
```

**Color lookup.** In P1, vertex color is derived from `WorldData` using
a simple priority order:

1. Ocean (height level 0) → deep blue `#1a3a5c`
2. Coast (height level 1) → shallow blue `#2e6ea6`
3. Mountain → dark grey `#5a5a5a`
4. Hilly → apply a darkening factor (~15%) to the biome color below
5. Biome:
   - `TropicalRainforest` → dark green `#2d6a2d`
   - `TemperateForest` → medium green `#4a7c4a`
   - `BorealForest` → muted green `#4a6b55`
   - `Grassland` → yellow-green `#7a9a3a`
   - `Desert` → sandy tan `#c8a96e`
   - `Tundra` → pale grey-green `#8a9a8a`

These colors are not art — they are debug-readable placeholders. The
color table lives in `terrain_color.rs` as a simple `match` expression,
making it trivial to update in P3.

River tiles receive a thin blue tint applied on top of their terrain
color in P1. In P3 this becomes a dedicated feature overlay (Layer 1).

### P1.2 Chunk spawning startup system

A Bevy `Startup` system in `chunk_spawner.rs` that runs once at app
launch:

1. Calls `worldgen::generate_world` with a hardcoded default seed and
   `WorldParams::default()`. In Stage 4, this will accept a seed from
   the game state; for the gate, hardcoded is sufficient.
2. Inserts `WorldData` as a Bevy `Resource`.
3. Iterates over all chunk origins: `q` from 0 to `ceil(width / CHUNK_SIZE)`,
   `r` from 0 to `ceil(height / CHUNK_SIZE)`.
4. For each chunk, calls `build_chunk_mesh`, registers the mesh with
   `Assets<Mesh>`, and spawns a `MaterialMeshBundle` entity.

The material is `StandardMaterial` with:

```rust
StandardMaterial {
    unlit: true,             // no lighting; vertex colors pass through
    vertex_colors: true,     // use the COLOR attribute
    cull_mode: None,         // both faces visible (safe default for 2D)
    ..Default::default()
}
```

`unlit: true` is correct for a 2D strategy map. The terrain is not a 3D
scene; there is no light source. This also removes lighting as a source
of GPU overhead.

### P1.3 Camera

An orthographic camera in `camera.rs` looking straight down at Z = 0.

**Projection:**

```rust
Camera2dBundle  // Bevy's built-in orthographic 2D camera
```

Bevy's `Camera2dBundle` uses an orthographic projection with Y-up,
which matches the hex coordinate layout (positive `r` is southward in
axial coords, which maps to negative Y in screen space — handle this
with a Y-flip in the hex center formula or by configuring the camera
projection accordingly).

**Pan input.** WASD or arrow keys translate the camera transform. Pan
speed is proportional to the current zoom level so the map doesn't feel
sluggish when zoomed out. A `PanSpeed` constant (world units per second
at zoom 1.0) is exposed as a tunable:

```rust
const PAN_SPEED: f32 = 500.0; // world units per second at zoom 1.0
```

**Zoom input.** Mouse scroll wheel changes the orthographic scale.
Clamped to a range that keeps the map readable:

```rust
const ZOOM_MIN: f32 = 0.5;   // zoomed in: tiles are large
const ZOOM_MAX: f32 = 8.0;   // zoomed out: full map visible at once
```

**East-west wrap.** In P1, the camera is clamped to the map bounds —
panning past the east edge stops at the edge. Wrap-aware camera
(duplicate geometry, seamless cylindrical pan) is Stage 4 work. This
is a known limitation documented in ADR-0018.

### P1.4 FPS overlay

Add `FrameTimeDiagnosticsPlugin` and a simple `TextBundle` that updates
every 0.5 seconds showing current FPS and frame time in milliseconds.
Position in the top-left corner, white text with a dark background for
readability against any terrain color.

### P1.5 Gate measurement

Pass criteria (all must hold):

- Minimum FPS ≥ 60 during continuous camera pan across the full map
  width, measured over at least two full traversals.
- No frame-time spike exceeds 2× the average frame time during pan.
- Memory footprint < 500 MB (measure via OS task manager or
  `heaptrack`).
- No Bevy validation warnings or errors at startup.

Record minimum, average, and maximum FPS and peak memory in ADR-0018.

### P1 file structure

```
crates/game/src/
├── main.rs                    ← Bevy App, plugin registration
└── rendering/
    ├── mod.rs                 ← RenderingPlugin: registers all systems
    ├── hex_mesh.rs            ← build_chunk_mesh(), CHUNK_SIZE
    ├── terrain_color.rs       ← WorldData → vertex color (P1), texture
    │                            index (P2+)
    ├── chunk_spawner.rs       ← Startup system: world gen + chunk spawn
    └── camera.rs              ← orthographic camera, pan/zoom input
```

### P1 gate: complete when

- App launches and renders 240×150 hex map as chunked meshes.
- FPS overlay visible and updating.
- All pass criteria in §P1.5 met and recorded.
- ADR-0018 written and accepted.
- `documents/architecture/architecture.md` updated: `game` crate no
  longer a stub.

---

## Stage P2: Texture pipeline

**Goal:** Replace vertex colors with a texture array material. Validate
that the textured pipeline meets the same 60 FPS target. Establish the
asset organization and shader infrastructure that P3's authored art will
slot into.

This stage uses placeholder textures — flat solid-color 64×64 PNGs, one
per terrain type and feature. No art skill required. The goal is a
working pipeline, not visual quality.

### P2.1 Texture array and atlas decision

**Decision: texture array.**

A texture array is a GPU resource that holds N textures at the same
resolution and format, addressable by integer index. The shader samples
it as `texture2DArray(texture, uv, layer_index)`. This is the correct
choice here for three reasons:

- Textures are authored independently — each terrain type is its own
  PNG file, not packed into a sheet. Adding a new terrain type is
  adding a file, not repacking an atlas.
- All terrain textures will be the same resolution (64×64 for
  placeholder, 128×128 or 256×256 for final art). Texture arrays
  require uniform dimensions; this constraint is acceptable and
  enforced at load time.
- The layer index is a small integer that fits cleanly into a vertex
  attribute. No UV offset arithmetic.

A texture atlas (one large spritesheet, UV offsets per tile) is the
alternative. It is rejected because atlas packing is fiddly, UV offset
arithmetic is error-prone, and adding a new terrain type requires
repacking the sheet and updating all UV mappings. The texture array
approach scales to 50+ terrain/feature types without any of this
overhead. See ADR-0019.

### P2.2 Custom material

`StandardMaterial` does not support texture arrays — it samples a single
`Texture2D`. A custom material is required.

Bevy's material system (`Material` trait + `AsBindGroup` derive) makes
this tractable. The custom material in `terrain_material.rs` binds:

- A `texture_2d_array` binding for the terrain texture array.
- A `sampler` binding for it.

The WGSL vertex shader passes the texture layer index from a vertex
attribute to the fragment shader. The fragment shader samples the array
and outputs the texel color. The full shader is approximately 40 lines
of WGSL — straightforward once the bind group layout is correct.

**Vertex attribute change from P1.** The `COLOR` attribute is replaced
by a `TEXTURE_INDEX` attribute (a `u32` per vertex, same value for all
7 vertices of a tile). The `build_chunk_mesh` function in `hex_mesh.rs`
gains this attribute; the `COLOR` attribute is removed. The change is
localized to `hex_mesh.rs` and `terrain_color.rs` — everything else is
unchanged.

### P2.3 UV coordinates

Each hex tile's vertices need UV coordinates for texture sampling. For a
hexagonal tile mapped to a square texture, the UVs follow the pointy-top
hex shape:

- The center vertex maps to `(0.5, 0.5)`.
- The six corner vertices map to the six equidistant points around the
  unit circle, scaled to the `[0, 1]` UV range.

Because the texture is sampled at the fragment level and the texture
index is uniform across the tile (every fragment of a grassland tile
samples the grassland texture), UV continuity across tiles is not
required. Each tile's UVs are independent and reset to the unit hex
layout.

The `TEXCOORD_0` attribute is added to the mesh alongside `POSITION`,
`NORMAL`, and `TEXTURE_INDEX`.

### P2.4 Placeholder textures

One PNG per entry in the texture index table. Resolution: 64×64 pixels.
Content: flat solid color matching the P1 vertex color for that terrain
type, with a subtle 4-pixel dark border to preserve the tile grid
appearance.

These are generated programmatically — a short script or build step
that writes the PNGs. No manual art work. The script lives in
`tools/gen_placeholder_textures/` and produces files into
`assets/textures/terrain/`.

**Texture index table (initial):**

| Index | Name | Color | Notes |
|---|---|---|---|
| 0 | `ocean` | `#1a3a5c` | Deep water |
| 1 | `coast` | `#2e6ea6` | Shallow water |
| 2 | `tropical_rainforest` | `#2d6a2d` | |
| 3 | `temperate_forest` | `#4a7c4a` | |
| 4 | `boreal_forest` | `#4a6b55` | |
| 5 | `grassland` | `#7a9a3a` | |
| 6 | `desert` | `#c8a96e` | |
| 7 | `tundra` | `#8a9a8a` | |
| 8 | `mountain` | `#5a5a5a` | |
| 9 | `ocean_hilly` | reserved | (unused; ocean has no hills) |
| 10–17 | `*_hilly` variants | darkened base | One per land biome + mountain |

The hilly variants are separate texture indices even in placeholder form,
because in P3 they will have distinct authored art (raised geometry
suggestion, different shading). Encoding them as separate indices now
avoids a texture pipeline change when P3 art arrives.

The index-to-terrain mapping lives in `terrain_color.rs`, which is
renamed to `terrain_lookup.rs` in P2 to reflect its expanded role.

### P2.5 Asset loading

Bevy's asset system loads the texture array at startup. The load is
async; the chunk spawner system must wait for the texture array to be
ready before spawning chunk entities. Use Bevy's `Handle<Image>` state
checking in a run condition or a simple state machine:

```
AppState::LoadingAssets  →  AppState::SpawningChunks  →  AppState::Running
```

This is the first use of `AppState` in the game crate — it will be
expanded significantly in Stage 4 when the full turn phase state machine
is implemented. Keep it minimal here: three variants, one transition
each.

### P2.6 Gate measurement

Same pass criteria as P1.5. The textured pipeline must match or beat the
vertex-color baseline. Record results in ADR-0019 alongside the texture
pipeline decision rationale.

### P2 file structure additions

```
crates/game/src/rendering/
├── terrain_lookup.rs          ← terrain → texture index (replaces terrain_color.rs)
└── terrain_material.rs        ← custom Material impl + WGSL shader

crates/game/src/
└── app_state.rs               ← AppState enum (LoadingAssets, SpawningChunks,
                                  Running)

assets/textures/terrain/
├── ocean.png                  ← 64×64 placeholder
├── coast.png
├── tropical_rainforest.png
├── ... (one per table entry above)
└── mountain_hilly.png

tools/gen_placeholder_textures/
└── src/main.rs                ← generates assets/textures/terrain/*.png
```

### P2 gate: complete when

- App renders map using texture array material.
- All placeholder textures visible and correctly assigned to terrain types.
- FPS target met (same criteria as P1).
- `AppState` loading sequence works: no entities spawn before textures load.
- ADR-0019 written and accepted.

---

## Stage P3: Terrain visuals

**Goal:** Replace placeholder textures with authored art sufficient for
development use. Every terrain type and major feature is visually
distinct and readable at all zoom levels. This is not final art — it
is the level of quality needed to build cities, units, and UI against
without being misled by the visuals.

### P3.1 Art requirements

The following assets are required. All are top-down hex tiles,
pointy-top orientation, 256×256 pixels, PNG with transparency for
corners outside the hex boundary.

**Terrain base tiles (Layer 0):**

| Asset | Description |
|---|---|
| `ocean.png` | Deep water — dark blue, subtle wave pattern |
| `coast.png` | Shallow water — lighter blue, suggests shallowing |
| `grassland.png` | Open plains — green, subtle grass texture |
| `grassland_hilly.png` | Grassland with visible elevation suggestion |
| `temperate_forest.png` | Deciduous tree canopy, top-down |
| `temperate_forest_hilly.png` | Forest on elevation |
| `boreal_forest.png` | Conifer canopy, darker green |
| `boreal_forest_hilly.png` | Boreal on elevation |
| `tropical_rainforest.png` | Dense canopy, rich green |
| `tropical_rainforest_hilly.png` | Tropical on elevation |
| `desert.png` | Sandy/rocky, warm tones |
| `desert_hilly.png` | Desert on elevation |
| `tundra.png` | Sparse ground cover, cold tones |
| `tundra_hilly.png` | Tundra on elevation |
| `mountain.png` | Impassable — clear mountain peak symbol |

**Feature overlay tiles (Layer 1):**

Layer 1 is a separate chunk mesh rendered on top of Layer 0 at a slight
Z offset. Feature tiles use transparency — only the feature itself is
opaque; the terrain underneath shows through.

| Asset | Description |
|---|---|
| `river.png` | Blue water channel, fits hex edge flow directions |
| `floodplain.png` | Subtle fertile-land tint overlay |
| `resource_iron.png` | Iron ore icon, top-down |
| `resource_horses.png` | Horse icon |
| `resource_gold.png` | Gold nugget icon |
| `resource_gems.png` | Gem icon |
| `resource_spices.png` | Spice plant icon |
| `resource_wine.png` | Grapevine icon |
| `resource_wheat.png` | Wheat sheaf icon |
| `resource_fish.png` | Fish icon (for coast/ocean tiles) |
| `resource_stone.png` | Stone icon |

Resource icons are small (centered in the hex, approximately 40% of
tile width) and rendered on top of the terrain texture. They do not
need to be photorealistic — a clean iconic silhouette is sufficient and
preferable for readability at small zoom levels.

### P3.2 Art sourcing

Three options in order of preference:

**Option A — CC0 asset packs.** Kenney.nl and itch.io host hex tile
packs under CC0 / public domain licenses. Check for packs with
top-down hex terrain in pointy-top orientation. This is the fastest
path if a suitable pack exists. Adapt colors and icons to match the
project's terrain type set.

**Option B — AI-generated textures.** Generate tile textures using
Stable Diffusion or FLUX (both runnable locally given the available
hardware). Prompt for top-down hex tiles, one terrain type at a time.
Use ComfyUI for batch generation and upscaling. This requires some
iteration to get consistent style across tile types but is faster than
manual authoring.

**Option C — Manual authoring.** Aseprite or similar pixel art tool.
Pixel art scales well at low resolution and reads clearly at all zoom
levels. A 256×256 hex tile in pixel art style is achievable without
professional art skill.

The three options are not mutually exclusive — a CC0 base pack adapted
with AI-generated or manual icons for the resource overlays is a
reasonable hybrid.

### P3.3 Feature overlay implementation

Layer 1 is built using the same chunk mesh infrastructure as Layer 0.
A second `build_feature_chunk_mesh` function in `hex_mesh.rs` produces
a mesh containing only tiles with features, using the feature texture
indices and transparency-enabled material.

Feature tiles are sparse — most tiles have no feature. The feature mesh
skips empty tiles entirely, so its vertex count is proportional to the
number of tiles with features, not the total tile count.

The feature material differs from the terrain material in one respect:
`alpha_mode: AlphaMode::Blend` to allow transparency. This is slightly
more expensive than opaque rendering but the feature layer is sparse,
so the cost is low.

River rendering in P3 replaces the P1 color tint and P2 placeholder
texture with directional river tiles — the river texture varies based
on which edges the river enters and exits. This requires encoding flow
direction into the feature mesh, which is read from
`HydrologyData::flow_direction`. There are at most 6 entry edges and 6
exit edges, giving a finite set of river tile variants (in practice
far fewer, since rivers have one entry and one exit). Store the variants
as additional texture array layers.

### P3.4 Resolution upgrade

When P3 art is ready, increase texture resolution from 64×64 (P2
placeholder) to 256×256 (P3 authored). The texture array is rebuilt;
no other code changes. Validate that FPS is unaffected — texture
sampling cost at 256×256 vs 64×64 is negligible on modern hardware.

### P3 gate: complete when

- All terrain base tile assets present and visible in-game.
- All feature overlay assets present: rivers, floodplains, resources.
- Every terrain type and feature is visually distinct and unambiguous
  at zoom levels 1 through 8.
- Rivers display directional variants correctly.
- FPS target maintained.
- Art source documented (license, origin, any adaptations) in
  `assets/textures/terrain/SOURCES.md`.

---

## Cross-cutting concerns

### Wrap-aware camera (deferred to Stage 4)

P1 clamps the camera to map bounds. Stage 4 implements the full
cylindrical wrap: a second copy of the terrain chunk meshes offset by
one map width, camera teleport when crossing the midpoint, seamless
east-west pan. This is a camera and chunk spawner change only — the
mesh building and material code are unaffected.

### Chunk invalidation (deferred to Stage 4+)

When a terrain-modifying event occurs (channel digging, deforestation),
the affected chunk's mesh is rebuilt by calling `build_chunk_mesh`
again for that chunk and swapping the mesh handle. This is a planned
extension of `chunk_spawner.rs`, not a new system. No architecture
change is needed; the chunk entity structure already supports mesh
handle replacement.

### Coordinate system note

The hex geometry in this plan uses the same axial coordinate convention
as the `coords` crate: `q` is east-west (wrapping), `r` is north-south
(bounded). The world-space Y axis is vertical on screen: positive `r`
is south (downward). Bevy's `Camera2dBundle` uses Y-up by default,
which means positive world Y is upward on screen. The hex center
formula must negate the Y component to align with Bevy's camera
convention:

```rust
let x = hex_size * (SQRT3 * q + SQRT3 / 2.0 * r);
let y = -(hex_size * (1.5 * r));  // negate: positive r is southward
```

This is the one place where the debug_viz and game rendering diverge —
debug PNGs use image-space (Y-down) while Bevy uses world-space (Y-up).

### ADRs to write

- **ADR-0018** (at P1 gate): Chunked hex mesh rendering approach.
  Records chunk size choice (16×16), the decision to start with vertex
  colors rather than textures, the measured performance results, and the
  known limitation of non-wrap-aware camera in P1.
- **ADR-0019** (at P2 gate): Texture array over texture atlas. Records
  the pipeline decision, texture resolution choice (64×64 placeholder,
  256×256 target), and the `AppState` loading sequence introduction.
