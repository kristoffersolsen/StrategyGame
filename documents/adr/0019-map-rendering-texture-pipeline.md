# ADR-0019: Map Rendering Texture Pipeline

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Kristoffer

## Context

Stage P2 replaces the P1 vertex-colour terrain renderer with a texture
pipeline that P3 authored terrain art can use without changing chunk
layout, camera behavior, or render-gate tooling.

The terrain layer remains static after world generation. The renderer
therefore needs a low-draw-call way to assign one terrain visual per hex
while preserving the existing 16x16 chunk mesh strategy from ADR-0018.

## Decision

Use a texture array for terrain tile art. Each terrain layer has the
same resolution and format, and chunk mesh vertices carry a `u32`
texture-layer attribute. The custom Bevy 2D material samples the array
in WGSL using that layer index and per-tile UV coordinates.

The alternative, a texture atlas, is rejected for terrain tiles because
it would require UV offset arithmetic and repacking when terrain variants
are added. Texture arrays match the intended asset model: one logical
layer per terrain or terrain-feature variant.

P2 uses generated 64x64 placeholder layers with the same colors as the
P1 debug renderer and a small dark border. The placeholder texture is
created as a Bevy `Image` asset at startup. P3 can replace that source
with authored texture files while keeping the mesh attribute, material,
shader, and renderer selection API stable.

Renderer startup now uses a minimal state sequence:

| State | Purpose |
|---|---|
| `LoadingAssets` | Prepare renderer-specific assets such as the texture array. |
| `SpawningChunks` | Generate `WorldData` and spawn terrain chunks, camera, and FPS overlay. |
| `Running` | Normal interactive render-gate state. |

`tools/maprender` owns the P2 executable gate through
`tools/maprender/presets/p2_texture_array.ron`. The production game uses
the same `game::rendering::RenderingPlugin` and can select either
`TerrainRenderer::VertexColor` or `TerrainRenderer::TextureArray`.

## Consequences

**Positive:**

- The P1 and P2 renderers are interchangeable through startup config and
  maprender presets.
- P3 can add authored terrain art without changing chunk topology.
- Texture selection is explicit and inspectable through
  `terrain_lookup.rs`.
- Draw-call count remains tied to chunk count, not tile count.

**Negative:**

- A custom material and WGSL shader are now part of the terrain pipeline.
- All terrain texture layers must share dimensions and format.
- The P2 placeholder source is generated in memory; P3 still needs the
  final file-backed art source and license documentation.

## Gate Measurement

Automated compile validation:

| Check | Result |
|---|---|
| `cargo check --workspace` | Pass |
| `cargo check -p game` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `cargo test -p maprender` | Pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `cargo run -p maprender -- --help` | Pass |

Interactive GPU measurements still need to be recorded from a local P2
run:

| Metric | Result |
|---|---|
| Minimum FPS during two full-width pans | Not recorded |
| Average FPS during two full-width pans | Not recorded |
| Maximum frame-time spike | Not recorded |
| Peak memory | Not recorded |
| Bevy startup validation warnings | Not recorded |
