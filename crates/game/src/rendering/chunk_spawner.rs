//! Startup system that generates the world and spawns terrain chunks.
#![allow(clippy::needless_pass_by_value)]

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    image::Image,
    prelude::{
        AlphaMode, AssetServer, Assets, Color, ColorMaterial, Commands, Component, Handle, Mesh,
        Mesh2d, Mesh3d, MeshMaterial2d, MeshMaterial3d, NextState, Res, ResMut, Resource,
        StandardMaterial, Transform, info,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use coords::{Coord, MapBounds};
use worldgen::{WorldData, generate_world};

use crate::{
    app_state::RenderAppState,
    rendering::{
        RenderStartupConfig, TerrainRenderer,
        feature_models::{FeatureModelAssets, spawn_feature_models},
        hex_mesh::{
            CHUNK_SIZE, build_terrain_3d_chunk_mesh, build_texture_chunk_mesh,
            build_vertex_color_chunk_mesh, build_water_plane_mesh, map_world_size,
        },
        terrain_lookup::{
            P3_TEXTURE_LAYER_COUNT, P3_TEXTURE_SIZE, TEXTURE_LAYER_COUNT, TEXTURE_SIZE,
            p3_texture_array_data, placeholder_texture_array_data,
        },
        terrain_material::{
            TerrainTextureArray3dMaterial, TerrainTextureArrayMaterial,
            configure_placeholder_texture,
        },
    },
};

/// Generated world data available to rendering and future game systems.
#[derive(Resource)]
pub(crate) struct GeneratedWorld {
    /// Generated world data consumed by rendering and future game systems.
    pub(crate) data: WorldData,
}

/// World-space dimensions of the static generated map.
#[derive(Resource, Clone, Copy)]
pub(crate) struct MapRenderBounds {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Marker for static terrain chunk entities.
#[derive(Component)]
struct TerrainChunk;

#[derive(Resource)]
pub(crate) struct TerrainTextureArray {
    handle: Handle<Image>,
}

#[derive(Resource)]
pub(crate) struct TerrainTextureArray3d {
    handle: Handle<Image>,
}

#[derive(SystemParam)]
pub(crate) struct TerrainMaterialAssets<'w> {
    color: ResMut<'w, Assets<ColorMaterial>>,
    standard: ResMut<'w, Assets<StandardMaterial>>,
    texture_array: ResMut<'w, Assets<TerrainTextureArrayMaterial>>,
    texture_array_3d: ResMut<'w, Assets<TerrainTextureArray3dMaterial>>,
}

#[derive(SystemParam)]
pub(crate) struct PreparedRenderAssets<'w> {
    texture_array: Option<Res<'w, TerrainTextureArray>>,
    texture_array_3d: Option<Res<'w, TerrainTextureArray3d>>,
    feature_models: Option<Res<'w, FeatureModelAssets>>,
}

pub(crate) fn prepare_render_assets(
    mut commands: Commands,
    config: Res<RenderStartupConfig>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    if config.terrain_renderer == TerrainRenderer::TextureArray {
        let mut texture = Image::new(
            Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: TEXTURE_LAYER_COUNT,
            },
            TextureDimension::D2,
            placeholder_texture_array_data(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        configure_placeholder_texture(&mut texture);
        commands.insert_resource(TerrainTextureArray {
            handle: images.add(texture),
        });
    }
    if config.terrain_renderer == TerrainRenderer::Terrain3d {
        commands.insert_resource(FeatureModelAssets::load(&asset_server));
        commands.insert_resource(TerrainTextureArray3d {
            handle: images.add(Image::new(
                Extent3d {
                    width: P3_TEXTURE_SIZE,
                    height: P3_TEXTURE_SIZE,
                    depth_or_array_layers: P3_TEXTURE_LAYER_COUNT,
                },
                TextureDimension::D2,
                p3_texture_array_data(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )),
        });
    } else {
        next_state.set(RenderAppState::SpawningChunks);
    }
}

pub(crate) fn finish_render_asset_loading(
    config: Res<RenderStartupConfig>,
    asset_server: Res<AssetServer>,
    feature_models: Option<Res<FeatureModelAssets>>,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    if config.terrain_renderer != TerrainRenderer::Terrain3d {
        return;
    }
    let Some(feature_models) = feature_models else {
        return;
    };
    if feature_models.all_loaded(&asset_server) {
        next_state.set(RenderAppState::SpawningChunks);
    }
}

pub(crate) fn spawn_chunks(
    mut commands: Commands,
    config: Res<RenderStartupConfig>,
    prepared_assets: PreparedRenderAssets,
    mut meshes: ResMut<Assets<Mesh>>,
    mut material_assets: TerrainMaterialAssets,
    mut next_state: ResMut<NextState<RenderAppState>>,
) {
    let world = generate_world(&config.world_params)
        .expect("render startup world generation parameters must be valid");
    let bounds = world.bounds;
    let (map_width, map_height) = map_world_size(bounds);

    match config.terrain_renderer {
        TerrainRenderer::VertexColor => {
            let material = material_assets.color.add(ColorMaterial::from(Color::WHITE));
            for origin in chunk_origins(bounds) {
                let mesh = build_vertex_color_chunk_mesh(origin, bounds, &world);
                commands.spawn((
                    TerrainChunk,
                    Mesh2d(meshes.add(mesh)),
                    MeshMaterial2d(material.clone()),
                    Transform::default(),
                ));
            }
        }
        TerrainRenderer::TextureArray => {
            let texture = prepared_assets
                .texture_array
                .as_ref()
                .expect("texture-array renderer assets must be prepared before chunk spawning")
                .handle
                .clone();
            let material = material_assets
                .texture_array
                .add(TerrainTextureArrayMaterial::from_texture(texture));

            for origin in chunk_origins(bounds) {
                let mesh = build_texture_chunk_mesh(origin, bounds, &world);
                commands.spawn((
                    TerrainChunk,
                    Mesh2d(meshes.add(mesh)),
                    MeshMaterial2d(material.clone()),
                    Transform::default(),
                ));
            }
        }
        TerrainRenderer::Terrain3d => {
            let texture = prepared_assets
                .texture_array_3d
                .as_ref()
                .expect("3D texture-array renderer assets must be prepared before chunk spawning")
                .handle
                .clone();
            let material = material_assets
                .texture_array_3d
                .add(TerrainTextureArray3dMaterial::from_texture(texture));
            let water_material = material_assets.standard.add(StandardMaterial {
                base_color: Color::srgba(0.16, 0.42, 0.70, 0.55),
                cull_mode: None,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.35,
                ..Default::default()
            });

            for origin in chunk_origins(bounds) {
                let mesh = build_terrain_3d_chunk_mesh(origin, bounds, &world);
                commands.spawn((
                    TerrainChunk,
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material.clone()),
                    Transform::default(),
                ));
            }

            commands.spawn((
                Mesh3d(meshes.add(build_water_plane_mesh(bounds))),
                MeshMaterial3d(water_material),
                Transform::default(),
            ));

            let feature_models = prepared_assets
                .feature_models
                .as_ref()
                .expect("feature model assets must be prepared before 3D chunk spawning");
            spawn_feature_models(&mut commands, feature_models, &world, bounds);
        }
    }

    info!(
        "spawned terrain chunks for {}x{} map",
        bounds.width(),
        bounds.height()
    );
    commands.insert_resource(MapRenderBounds {
        width: map_width,
        height: map_height,
    });
    commands.insert_resource(GeneratedWorld { data: world });
    next_state.set(RenderAppState::Running);
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
