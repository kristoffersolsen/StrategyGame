//! Chunked pointy-top hex mesh construction.

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::Mesh,
    render::render_resource::PrimitiveTopology,
};
use coords::{Coord, MapBounds};
use worldgen::WorldData;

use crate::rendering::terrain_color;

/// Tile count per chunk side. A chunk covers `CHUNK_SIZE x CHUNK_SIZE` tiles.
///
/// Sixteen keeps the default 240x150 map near 150 chunk entities while
/// preserving enough granularity for future frustum culling.
pub(crate) const CHUNK_SIZE: u32 = 16;

/// Circumradius of one pointy-top hex in world units.
pub(crate) const HEX_SIZE: f32 = 16.0;

/// Builds a static vertex-colour mesh for one terrain chunk.
pub(crate) fn build_vertex_color_chunk_mesh(
    chunk_origin: Coord,
    bounds: MapBounds,
    world: &WorldData,
) -> Mesh {
    let max_q = (chunk_origin.q + CHUNK_SIZE.cast_signed()).min(bounds.width().cast_signed());
    let max_r = (chunk_origin.r + CHUNK_SIZE.cast_signed()).min(bounds.height().cast_signed());
    let tile_count = usize::try_from((max_q - chunk_origin.q) * (max_r - chunk_origin.r))
        .expect("chunk tile count must be non-negative");

    let mut positions = Vec::with_capacity(tile_count * 7);
    let mut normals = Vec::with_capacity(tile_count * 7);
    let mut colors = Vec::with_capacity(tile_count * 7);
    let mut indices = Vec::with_capacity(tile_count * 18);
    let mountain_level = mountain_level(world);

    for r in chunk_origin.r..max_r {
        for q in chunk_origin.q..max_q {
            let coord = bounds
                .normalize(Coord::new(q, r))
                .expect("chunk iteration keeps r in bounds");
            push_hex(
                coord,
                bounds,
                terrain_color::tile_color(world, coord, mountain_level),
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
            );
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Returns the approximate world-space size of the generated map.
pub(crate) fn map_world_size(bounds: MapBounds) -> (f32, f32) {
    let width = f32::sqrt(3.0) * HEX_SIZE * (u32_to_f32(bounds.width()) + 1.0);
    let height = HEX_SIZE * (1.5 * (u32_to_f32(bounds.height()) - 1.0) + 2.0);
    (width, height)
}

fn push_hex(
    coord: Coord,
    bounds: MapBounds,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let center = hex_center(coord, bounds);
    let start = u32::try_from(positions.len()).expect("chunk mesh vertex count fits in u32");

    positions.push([center[0], center[1], 0.0]);
    normals.push([0.0, 0.0, 1.0]);
    colors.push(color);

    for angle in CORNER_ANGLES {
        positions.push([
            center[0] + HEX_SIZE * angle.cos(),
            center[1] + HEX_SIZE * angle.sin(),
            0.0,
        ]);
        normals.push([0.0, 0.0, 1.0]);
        colors.push(color);
    }

    for corner in 0..6 {
        let a = start + 1 + corner;
        let b = start + 1 + ((corner + 1) % 6);
        indices.extend_from_slice(&[start, a, b]);
    }
}

fn hex_center(coord: Coord, bounds: MapBounds) -> [f32; 2] {
    let q = i32_to_f32(coord.q);
    let r = i32_to_f32(coord.r);
    let wrapped_x = (q + 0.5 * r).rem_euclid(u32_to_f32(bounds.width()));
    [
        HEX_SIZE * f32::sqrt(3.0) * wrapped_x + HEX_SIZE * f32::sqrt(3.0) * 0.5,
        -HEX_SIZE * (1.5 * r + 1.0),
    ]
}

const CORNER_ANGLES: [f32; 6] = [
    PI / 6.0,
    PI / 2.0,
    5.0 * PI / 6.0,
    7.0 * PI / 6.0,
    3.0 * PI / 2.0,
    11.0 * PI / 6.0,
];

#[allow(clippy::cast_precision_loss)]
fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

#[allow(clippy::cast_precision_loss)]
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

fn mountain_level(world: &WorldData) -> u8 {
    world
        .tectonics
        .height_level
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
}
