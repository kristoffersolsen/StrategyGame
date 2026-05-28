//! Static world-map rendering.

mod camera;
mod chunk_spawner;
mod feature_models;
mod hex_mesh;
mod lighting;
mod terrain_lookup;
mod terrain_material;

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    pbr::MaterialPlugin,
    prelude::{
        App, AppExtStates, IntoScheduleConfigs, OnEnter, Plugin, Resource, Update, in_state,
    },
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
    /// P3 renderer: terrain chunks render through Bevy's 3D PBR mesh path.
    Terrain3d,
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
            .add_plugins(MaterialPlugin::<
                terrain_material::TerrainTextureArray3dMaterial,
            >::default())
            .init_state::<RenderAppState>()
            .init_resource::<RenderStartupConfig>()
            .init_resource::<camera::FpsOverlayTimer>()
            .add_systems(
                OnEnter(RenderAppState::LoadingAssets),
                chunk_spawner::prepare_render_assets,
            )
            .add_systems(
                Update,
                chunk_spawner::finish_render_asset_loading
                    .run_if(in_state(RenderAppState::LoadingAssets)),
            )
            .add_systems(
                OnEnter(RenderAppState::SpawningChunks),
                (
                    chunk_spawner::spawn_chunks,
                    camera::spawn_map_camera,
                    lighting::spawn_lighting,
                    camera::spawn_fps_overlay,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    camera::pan_camera,
                    camera::zoom_camera,
                    camera::update_orbit_camera,
                )
                    .run_if(in_state(RenderAppState::Running)),
            )
            .add_systems(
                Update,
                (camera::update_fps_overlay,).run_if(in_state(RenderAppState::Running)),
            );
    }
}
