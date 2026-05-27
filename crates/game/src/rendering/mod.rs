//! Static world-map rendering.

mod camera;
mod chunk_spawner;
mod hex_mesh;
mod terrain_color;

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::{App, IntoScheduleConfigs, Plugin, Resource, Startup, Update},
};
use worldgen::WorldParams;

/// Selects the terrain renderer implementation used for static terrain chunks.
///
/// The current implementation is the P1 vertex-colour renderer. Texture-based
/// variants are represented here now so render-gate presets, the tool CLI, and
/// the production game can switch renderer families without changing startup
/// plumbing as later stages land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainRenderer {
    /// P1 renderer: terrain identity is encoded directly as mesh vertex colour.
    #[default]
    VertexColor,
    /// Reserved for P2: texture-array terrain material.
    TextureArray,
}

/// Startup inputs for static map rendering.
#[derive(Resource, Debug, Clone)]
pub struct RenderStartupConfig {
    /// World-generation parameters used to create the render test map.
    pub world_params: WorldParams,
    /// Terrain renderer implementation to use.
    pub terrain_renderer: TerrainRenderer,
}

impl Default for RenderStartupConfig {
    fn default() -> Self {
        Self {
            world_params: WorldParams {
                seed: 1,
                ..WorldParams::default()
            },
            terrain_renderer: TerrainRenderer::VertexColor,
        }
    }
}

/// Registers terrain rendering, camera controls, and diagnostics overlay.
pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<RenderStartupConfig>()
            .init_resource::<camera::FpsOverlayTimer>()
            .add_systems(
                Startup,
                (
                    chunk_spawner::spawn_chunks,
                    camera::spawn_map_camera,
                    camera::spawn_fps_overlay,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    camera::pan_camera,
                    camera::zoom_camera,
                    camera::update_fps_overlay,
                ),
            );
    }
}
