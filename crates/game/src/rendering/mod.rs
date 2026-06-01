//! Static world-map rendering.

mod camera;
mod chunk_spawner;
pub(crate) mod hex_mesh;
mod lighting;
mod terrain_layer;
pub mod terrain_surface;

pub(crate) use hex_mesh::HEX_SIZE;

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::{
        App, AppExtStates, IntoScheduleConfigs, OnEnter, Plugin, Resource, Update, in_state,
    },
};

use crate::app_state::RenderAppState;

/// Startup inputs for static map rendering.
#[derive(Resource, Debug, Clone, Default)]
pub struct RenderStartupConfig;

/// Registers terrain rendering, camera controls, and diagnostics overlay.
pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(terrain_layer::TerrainLayerPlugin)
            .init_state::<RenderAppState>()
            .init_resource::<RenderStartupConfig>()
            .init_resource::<camera::FpsOverlayTimer>()
            .add_systems(
                OnEnter(RenderAppState::SpawningChunks),
                (
                    camera::spawn_map_camera,
                    lighting::spawn_lighting,
                    camera::spawn_fps_overlay,
                    camera::spawn_coord_overlay,
                    camera::spawn_turn_overlay,
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
                (
                    camera::update_fps_overlay,
                    camera::update_coord_overlay,
                    camera::update_turn_overlay,
                )
                    .run_if(in_state(RenderAppState::Running)),
            );
    }
}
