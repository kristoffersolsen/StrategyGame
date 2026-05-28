//! Static world-map rendering.

mod camera;
mod chunk_spawner;
mod hex_mesh;
mod terrain_lookup;
mod terrain_material;

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::{App, AppExtStates, IntoScheduleConfigs, OnEnter, Plugin, Resource, Update},
    sprite_render::Material2dPlugin,
};
use worldgen::WorldParams;

use crate::app_state::RenderAppState;

/// Selects the terrain renderer implementation used for static terrain chunks.
///
/// Render-gate presets, the tool CLI, and the production game all use this
/// enum so renderer families can be switched without changing startup plumbing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainRenderer {
    /// P1 renderer: terrain identity is encoded directly as mesh vertex colour.
    #[default]
    VertexColor,
    /// P2 renderer: terrain identity is encoded as texture-array layer indices.
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
        terrain_material::load_shader(app);

        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(Material2dPlugin::<
                terrain_material::TerrainTextureArrayMaterial,
            >::default())
            .init_state::<RenderAppState>()
            .init_resource::<RenderStartupConfig>()
            .init_resource::<camera::FpsOverlayTimer>()
            .add_systems(
                OnEnter(RenderAppState::LoadingAssets),
                chunk_spawner::prepare_render_assets,
            )
            .add_systems(
                OnEnter(RenderAppState::SpawningChunks),
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
