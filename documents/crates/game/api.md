# game public API

Public interface of the `game` crate. This file is thin — the game crate is ~30% complete.
Expand it as new pub items land in `crates/game/src/lib.rs`.

---

## rendering module

### RenderingPlugin

```rust
pub struct RenderingPlugin;  // implements bevy::prelude::Plugin
```

Registers: chunked terrain surface rendering, camera controls, lighting, and
FPS overlay diagnostics. Add to a Bevy `App` before calling `app.run()`.

Expects `RenderStartupConfig` to be inserted as a resource before `SpawningChunks` state
is entered. `init_resource::<RenderStartupConfig>()` is called by the plugin, so the
default is used if nothing is inserted explicitly.

### RenderStartupConfig

```rust
#[derive(Resource)]
pub struct RenderStartupConfig;
```

---

## State machine

`app_state` is private. Rendering progresses through `RenderAppState`:
`LoadingAssets` → `SpawningChunks` → `Running`. This is an implementation detail;
external callers only need to insert `RenderStartupConfig` before the plugin starts.
