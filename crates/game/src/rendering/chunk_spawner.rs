//! Terrain-layer startup scaffolding.
#![allow(clippy::needless_pass_by_value)]

use bevy::{
    asset::AssetServer,
    image::Image,
    prelude::{Commands, Handle, NextState, Res, ResMut, Resource, info},
};

use crate::{app_state::RenderAppState, rendering::hex_mesh::map_world_size, sim::StaticTerrain};

/// World-space dimensions of the static generated map.
#[derive(Resource, Clone, Copy)]
pub(crate) struct MapRenderBounds {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Texture handles reserved for the replacement terrain material.
#[derive(Resource)]
pub(crate) struct TerrainTextureAssets {
    layers: Vec<Handle<Image>>,
}

impl TerrainTextureAssets {
    fn load(asset_server: &AssetServer) -> Self {
        const LAYERS: [&str; 9] = [
            "worldgen/textures/ocean.png",
            "worldgen/textures/coast.png",
            "worldgen/textures/tropical.png",
            "worldgen/textures/temperate.png",
            "worldgen/textures/boreal.png",
            "worldgen/textures/grassland.png",
            "worldgen/textures/desert.png",
            "worldgen/textures/tundra.png",
            "worldgen/textures/mountain.png",
        ];

        Self {
            layers: LAYERS
                .into_iter()
                .map(|path| asset_server.load(path))
                .collect(),
        }
    }

    fn all_loaded(&self, asset_server: &AssetServer) -> bool {
        let mut loaded = true;
        for handle in &self.layers {
            let Some(state) = asset_server.get_recursive_dependency_load_state(handle) else {
                loaded = false;
                continue;
            };
            assert!(
                !state.is_failed(),
                "terrain texture asset failed to load; see Bevy asset errors above"
            );
            loaded &= state.is_loaded();
        }
        loaded
    }
}

pub(crate) fn prepare_render_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TerrainTextureAssets::load(&asset_server));
}

pub(crate) fn finish_render_asset_loading(
    asset_server: Res<AssetServer>,
    textures: Res<TerrainTextureAssets>,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    if textures.all_loaded(&asset_server) {
        next_state.set(RenderAppState::SpawningChunks);
    }
}

pub(crate) fn spawn_chunks(
    mut commands: Commands,
    terrain: Res<StaticTerrain>,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    let bounds = terrain.bounds;
    let (map_width, map_height) = map_world_size(bounds);
    commands.insert_resource(MapRenderBounds {
        width: map_width,
        height: map_height,
    });

    info!(
        width = bounds.width(),
        height = bounds.height(),
        "terrain layer scaffold active; no terrain mesh emitted"
    );
    next_state.set(RenderAppState::Running);
}
