# game public API

Public interface of the `game` crate. This file is thin — the game crate is ~30% complete.
Expand it as new pub items land in `crates/game/src/lib.rs`.

---

## rendering module

### RenderingPlugin

```rust
pub struct RenderingPlugin;  // implements bevy::prelude::Plugin
```

Registers: terrain material setup, camera controls (pan/zoom/orbit), chunk spawning,
lighting, FPS overlay diagnostics. Add to a Bevy `App` before calling `app.run()`.

Expects `RenderStartupConfig` to be inserted as a resource before `SpawningChunks` state
is entered. `init_resource::<RenderStartupConfig>()` is called by the plugin, so the
default is used if nothing is inserted explicitly.

### TerrainRenderer

```rust
pub enum TerrainRenderer {
    VertexColor,   // P1: terrain identity as mesh vertex colour. Default.
    TextureArray,  // P2: terrain identity as texture-array layer indices
    Terrain3d,     // P3: terrain chunks through Bevy 3D PBR mesh path
}
```

### RenderStartupConfig

```rust
#[derive(Resource)]
pub struct RenderStartupConfig {
    pub world_params: WorldParams,        // which world to generate and render
    pub terrain_renderer: TerrainRenderer,
}
// Default: seed=1, Medium map, VertexColor renderer
```

---

## State machine

`app_state` is private. Rendering progresses through `RenderAppState`:
`LoadingAssets` → `SpawningChunks` → `Running`. This is an implementation detail;
external callers only need to insert `RenderStartupConfig` before the plugin starts.
