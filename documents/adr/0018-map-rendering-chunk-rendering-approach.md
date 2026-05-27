# ADR-0018: Chunked Map Rendering Approach

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

The generated world map is static at game start and the default size is
240x150, approximately 36K tiles. Rendering one entity per tile would
produce unnecessary ECS overhead and draw-call pressure before any
dynamic game systems exist. Rendering the whole map as a single mesh
would be simple, but gives poor culling granularity and makes later
partial rebuilds awkward.

Stage P1 needs a performance baseline before the texture pipeline and
gameplay entities are built on top of the terrain layer.

## Decision

Render terrain as static chunk meshes. Each chunk covers 16x16 tiles and
is spawned as one Bevy entity, giving about 150 terrain entities for the
default map size. Each tile is triangulated as a pointy-top hex fan:
one center vertex, six corner vertices, and six triangles.

For P1, terrain identity is encoded as vertex colour. The colour table is
a debug-readable placeholder lookup derived from height level, biome,
hills, and river presence. It is intentionally isolated in
`terrain_color.rs` so P2 can replace the colour output with texture
indices without changing chunk iteration or camera systems.

Renderer selection is a startup configuration concern, not a hardcoded
game-main concern. `game::rendering::RenderStartupConfig` carries the
world-generation parameters and `TerrainRenderer` choice. The production
game can insert a config from game state later, while `tools/maprender`
loads one from editable RON presets for performance-gate runs.

The implementation uses Bevy 0.18's 2D mesh renderer:

- `Mesh2d` chunk components.
- `MeshMaterial2d<ColorMaterial>` with vertex colours.
- `Camera2d` for the orthographic map view.

The original plan described `MaterialMeshBundle` and `StandardMaterial`.
Those names belong to older Bevy APIs. The 2D path is the correct Bevy
0.18 equivalent for a flat strategy map and avoids introducing 3D
lighting, PBR, and a 3D camera only to render unlit terrain.

East-west camera wrapping is deferred. The P1 camera clamps to the
generated map bounds; seamless cylindrical panning will be handled when
gameplay camera behavior is designed.

The P1 executable gate is `tools/maprender`, not `crates/game/src/main.rs`.
`maprender` currently supports `renderer: VertexColor` and reserves
`renderer: TextureArray` for P2.

## Consequences

**Positive:**

- Draw-call count remains small at the target map size.
- Terrain rendering is static and independent from future units, cities,
  selection overlays, and fog-of-war layers.
- Chunks give a natural rebuild boundary for future terrain mutations.
- Bevy's 2D mesh path keeps the renderer aligned with the flat map
  presentation.

**Negative:**

- Seamless east-west panning requires future duplicate geometry or
  wrap-aware camera logic.
- P1 colours are debug placeholders, not art.
- Texture array support still requires a custom material in P2.

## Gate Measurement

Automated compile validation:

| Check | Result |
|---|---|
| `cargo check --workspace` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo clippy -p game --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `cargo test -p maprender` | Pass |
| `cargo test -p game --no-run` | Pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p game --no-deps` | Pass |
| `cargo run -p maprender -- --help` | Pass |
| `cargo check -p game` | Pass |

Interactive GPU measurements still need to be recorded from a local run:

| Metric | Result |
|---|---|
| Minimum FPS during two full-width pans | Not recorded |
| Average FPS during two full-width pans | Not recorded |
| Maximum frame-time spike | Not recorded |
| Peak memory | Not recorded |
| Bevy startup validation warnings | Not recorded |
