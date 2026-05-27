//! Startup system that generates the world and spawns terrain chunks.
#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::{
    Assets, Color, ColorMaterial, Commands, Component, Mesh, Mesh2d, MeshMaterial2d, ResMut,
    Resource, Transform, info,
};
use coords::{Coord, MapBounds};
use worldgen::{WorldData, generate_world};

use crate::rendering::{
    RenderStartupConfig, TerrainRenderer,
    hex_mesh::{CHUNK_SIZE, build_vertex_color_chunk_mesh, map_world_size},
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

pub(crate) fn spawn_chunks(
    mut commands: Commands,
    config: bevy::prelude::Res<RenderStartupConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let world = generate_world(&config.world_params)
        .expect("render startup world generation parameters must be valid");
    let bounds = world.bounds;
    let (map_width, map_height) = map_world_size(bounds);

    match config.terrain_renderer {
        TerrainRenderer::VertexColor => {
            let material = materials.add(ColorMaterial::from(Color::WHITE));
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
            panic!("texture-array terrain renderer is reserved for P2 and is not implemented yet");
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
