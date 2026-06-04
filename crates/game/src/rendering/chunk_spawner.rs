//! Terrain-layer startup scaffolding.
#![allow(clippy::needless_pass_by_value)]

use bevy::{
    asset::{AssetServer, RenderAssetUsages},
    image::{Image, ImageSampler},
    prelude::{
        Assets, Commands, Component, Handle, Mesh, Mesh3d, MeshMaterial3d, NextState, Res, ResMut,
        Resource, Transform, info,
    },
    render::render_resource::{
        Extent3d, TextureDimension, TextureViewDescriptor, TextureViewDimension,
    },
};
use coords::{Coord, MapBounds};

use crate::{
    app_state::RenderAppState,
    rendering::{
        hex_mesh::map_world_size,
        terrain_material::{TerrainMaterial, TerrainMaterialParams},
        terrain_surface::{CHUNK_SIZE, TerrainMeshParams, build_terrain_chunk_mesh},
    },
    sim::StaticTerrain,
};

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
    array_texture: Option<Handle<Image>>,
}

/// Marker for static terrain chunk entities.
#[derive(Component)]
struct TerrainChunk;

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
            array_texture: None,
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

    fn array_texture(&self) -> Handle<Image> {
        self.array_texture
            .as_ref()
            .expect("terrain array texture is prepared before chunk spawning")
            .clone()
    }
}

pub(crate) fn prepare_render_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TerrainTextureAssets::load(&asset_server));
}

pub(crate) fn finish_render_asset_loading(
    asset_server: Res<AssetServer>,
    mut textures: ResMut<TerrainTextureAssets>,
    mut images: ResMut<Assets<Image>>,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    if textures.all_loaded(&asset_server) {
        if textures.array_texture.is_none() {
            let array_texture = build_texture_array(&textures.layers, &images);
            textures.array_texture = Some(images.add(array_texture));
        }
        next_state.set(RenderAppState::SpawningChunks);
    }
}

pub(crate) fn spawn_chunks(
    mut commands: Commands,
    terrain: Res<StaticTerrain>,
    textures: Res<TerrainTextureAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    let bounds = terrain.bounds;
    let (map_width, map_height) = map_world_size(bounds);
    let material = materials.add(TerrainMaterial {
        texture_array: textures.array_texture(),
        params: TerrainMaterialParams::default(),
    });

    for origin in chunk_origins(bounds) {
        let mesh = build_terrain_chunk_mesh(origin, &terrain, TerrainMeshParams::default());
        commands.spawn((
            TerrainChunk,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform::default(),
        ));
    }

    commands.insert_resource(MapRenderBounds {
        width: map_width,
        height: map_height,
    });

    info!(
        width = bounds.width(),
        height = bounds.height(),
        "spawned continuous terrain surface chunks"
    );
    next_state.set(RenderAppState::Running);
}

fn build_texture_array(layer_handles: &[Handle<Image>], images: &Assets<Image>) -> Image {
    let first = layer_handles
        .first()
        .and_then(|handle| images.get(handle))
        .expect("terrain texture array requires at least one loaded layer");
    let size = first.texture_descriptor.size;
    let format = first.texture_descriptor.format;
    let layer_data_len = first
        .data
        .as_ref()
        .expect("loaded terrain texture has CPU pixel data")
        .len();
    let mut data = Vec::with_capacity(layer_data_len * layer_handles.len());

    for handle in layer_handles {
        let image = images
            .get(handle)
            .expect("terrain texture layer is loaded before array build");
        assert_eq!(
            image.texture_descriptor.size.width, size.width,
            "terrain texture layers must share width"
        );
        assert_eq!(
            image.texture_descriptor.size.height, size.height,
            "terrain texture layers must share height"
        );
        assert_eq!(
            image.texture_descriptor.format, format,
            "terrain texture layers must share format"
        );
        let layer_data = image
            .data
            .as_ref()
            .expect("loaded terrain texture has CPU pixel data");
        assert_eq!(
            layer_data.len(),
            layer_data_len,
            "terrain texture layers must share byte size"
        );
        data.extend_from_slice(layer_data);
    }

    let mut image = Image::new(
        Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: u32::try_from(layer_handles.len())
                .expect("terrain texture layer count fits in u32"),
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    });
    image
}

fn chunk_origins(bounds: MapBounds) -> impl Iterator<Item = Coord> {
    let chunk_columns = bounds.width().div_ceil(CHUNK_SIZE);
    let chunk_rows = bounds.height().div_ceil(CHUNK_SIZE);
    (0..chunk_rows).flat_map(move |row| {
        (0..chunk_columns).map(move |column| {
            Coord::new(
                (column * CHUNK_SIZE).cast_signed(),
                (row * CHUNK_SIZE).cast_signed(),
            )
        })
    })
}
