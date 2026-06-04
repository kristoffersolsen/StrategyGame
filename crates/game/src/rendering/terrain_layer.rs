//! Static terrain render layer plugin.

use bevy::{
    pbr::MaterialPlugin,
    prelude::{App, IntoScheduleConfigs, OnEnter, Plugin, Update, in_state},
};

use crate::{
    app_state::RenderAppState,
    rendering::{chunk_spawner, terrain_material::TerrainMaterial},
};

/// Registers the terrain layer's asset loading and rebuild hooks.
pub(crate) struct TerrainLayerPlugin;

impl Plugin for TerrainLayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                OnEnter(RenderAppState::LoadingAssets),
                chunk_spawner::prepare_render_assets,
            )
            .add_systems(
                Update,
                chunk_spawner::finish_render_asset_loading
                    .run_if(in_state(RenderAppState::LoadingAssets)),
            );
    }
}
