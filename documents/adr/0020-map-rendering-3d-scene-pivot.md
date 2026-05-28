# ADR-0020: Map Rendering 3D Scene Pivot

**Status:** Accepted  
**Date:** 2026-05-28  
**Deciders:** Kristoffer

## Context

P1 proved chunked flat vertex-colour terrain. P2 proved the texture-array
pipeline. P3 pivots the runtime map renderer to a 3D scene before cities,
units, and dynamic overlays are built on top of the terrain.

The world-generation pipeline already produces meaningful elevation:
ocean, coast, land, hills, and mountains. A flat top-down view can only
communicate that through color. The intended long-term direction needs
elevation, terrain features, models, and a tilted camera to read clearly.

## Decision

Add the 3D renderer as a new `TerrainRenderer::Terrain3d` variant rather
than replacing the P1/P2 renderers. This keeps old render gates available
for baseline comparison while the P3 path evolves.

P3.1 switches the selected renderer to Bevy's 3D path:

- `Mesh3d` terrain chunk entities.
- `MeshMaterial3d<StandardMaterial>` with vertex colours multiplied by
  a white lit material.
- `Camera3d` perspective view looking at the map center.
- `DirectionalLight`, `GlobalAmbientLight`, and sky `ClearColor`.

P3.2 adds the first 3D terrain geometry pass:

- Terrain top faces are raised to discrete surface tiers derived from
  `WorldData` height level and hill flags.
- Mountain tile centers are raised above their top-face corners to form
  a readable peak.
- Side faces are emitted where a tile is higher than its neighbour,
  including north/south map edges.
- A single transparent water plane sits just above coast level to show
  the waterline without per-tile water geometry.

P3.3 adds the orbit camera described in ADR-0021. Authored textures and
model instancing are later P3 steps.

## Consequences

**Positive:**

- P1, P2, and P3 renderers can be selected from `maprender` presets.
- The game crate now has the PBR feature path needed for terrain and
  later `.glb` models.
- The first P3 step isolates the scene-pipeline change from the upcoming
  mesh-geometry change.

**Negative:**

- Enabling `bevy_pbr` increases compile time and dependency surface.
- P3.2 still uses debug vertex colors; authored terrain textures return
  in later P3 texture work.
- The orbit camera is intentionally custom and will need later tuning for
  terrain-following and cylindrical wrap behavior.

## Gate Measurement

Automated validation:

| Check | Result |
|---|---|
| `cargo check --workspace` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `cargo test -p maprender` | Pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `cargo run -p maprender -- --help` | Pass |

Interactive P3 measurements still need to be recorded from a local run:

| Metric | Result |
|---|---|
| Minimum FPS during camera movement | Not recorded |
| Average FPS during camera movement | Not recorded |
| Maximum frame-time spike | Not recorded |
| Peak memory | Not recorded |
| Bevy startup validation warnings | Not recorded |
