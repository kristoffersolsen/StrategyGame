//! Chunked pointy-top hex mesh construction.

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, MeshVertexAttribute},
    prelude::Mesh,
    render::render_resource::{PrimitiveTopology, VertexFormat},
};
use coords::{Coord, Direction, MapBounds};
use worldgen::WorldData;

use crate::rendering::terrain_lookup;

/// Tile count per chunk side. A chunk covers `CHUNK_SIZE x CHUNK_SIZE` tiles.
///
/// Sixteen keeps the default 240x150 map near 150 chunk entities while
/// preserving enough granularity for future frustum culling.
pub(crate) const CHUNK_SIZE: u32 = 16;

/// Circumradius of one pointy-top hex in world units.
pub(crate) const HEX_SIZE: f32 = 16.0;

const OCEAN_HEIGHT: f32 = -8.0;
const COAST_HEIGHT: f32 = 0.0;
const LAND_HEIGHT: f32 = 4.0;
const HILL_HEIGHT: f32 = 12.0;
const MOUNTAIN_HEIGHT: f32 = 24.0;
const MOUNTAIN_PEAK_OFFSET: f32 = 16.0;
const WATER_PLANE_Z: f32 = 0.25;

/// Per-vertex terrain texture-array layer index for the P2 renderer.
pub(crate) const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("TerrainTextureIndex", 98_754_091_700, VertexFormat::Uint32);

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
                terrain_lookup::tile_color(world, coord, mountain_level),
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

/// Builds a static texture-array mesh for one terrain chunk.
pub(crate) fn build_texture_chunk_mesh(
    chunk_origin: Coord,
    bounds: MapBounds,
    world: &WorldData,
) -> Mesh {
    let max_q = (chunk_origin.q + CHUNK_SIZE.cast_signed()).min(bounds.width().cast_signed());
    let max_r = (chunk_origin.r + CHUNK_SIZE.cast_signed()).min(bounds.height().cast_signed());
    let tile_count = usize::try_from((max_q - chunk_origin.q) * (max_r - chunk_origin.r))
        .expect("chunk tile count must be non-negative");

    let mut mesh_data = TexturedMeshData::with_capacity(tile_count);
    let mountain_level = mountain_level(world);

    for r in chunk_origin.r..max_r {
        for q in chunk_origin.q..max_q {
            let coord = bounds
                .normalize(Coord::new(q, r))
                .expect("chunk iteration keeps r in bounds");
            mesh_data.push_hex(
                coord,
                bounds,
                terrain_lookup::texture_index(world, coord, mountain_level),
            );
        }
    }

    mesh_data.into_mesh()
}

/// Builds a static P3 terrain mesh with elevation, side faces, and mountain peaks.
pub(crate) fn build_terrain_3d_chunk_mesh(
    chunk_origin: Coord,
    bounds: MapBounds,
    world: &WorldData,
) -> Mesh {
    let max_q = (chunk_origin.q + CHUNK_SIZE.cast_signed()).min(bounds.width().cast_signed());
    let max_r = (chunk_origin.r + CHUNK_SIZE.cast_signed()).min(bounds.height().cast_signed());
    let tile_count = usize::try_from((max_q - chunk_origin.q) * (max_r - chunk_origin.r))
        .expect("chunk tile count must be non-negative");

    let mut mesh_data = Terrain3dMeshData::with_capacity(tile_count);
    let mountain_level = mountain_level(world);

    for r in chunk_origin.r..max_r {
        for q in chunk_origin.q..max_q {
            let coord = bounds
                .normalize(Coord::new(q, r))
                .expect("chunk iteration keeps r in bounds");
            let surface = terrain_surface(world, coord, mountain_level);
            let texture_index = terrain_lookup::p3_texture_index(world, coord, mountain_level);
            let is_mountain = is_mountain(world, coord, mountain_level);

            mesh_data.push_top(coord, bounds, surface, texture_index, is_mountain);
            for direction in Direction::ALL {
                let neighbor_surface = bounds
                    .neighbor(coord, direction)
                    .map_or(OCEAN_HEIGHT, |neighbor| {
                        terrain_surface(world, neighbor, mountain_level)
                    });
                if neighbor_surface < surface {
                    mesh_data.push_side(
                        coord,
                        bounds,
                        direction,
                        neighbor_surface,
                        surface,
                        texture_index,
                    );
                }
            }
        }
    }

    mesh_data.into_mesh()
}

/// Builds the P3 water plane at coast level.
pub(crate) fn build_water_plane_mesh(bounds: MapBounds) -> Mesh {
    let (width, height) = map_world_size(bounds);
    let positions = vec![
        [0.0, 0.0, WATER_PLANE_Z],
        [width, 0.0, WATER_PLANE_Z],
        [width, -height, WATER_PLANE_Z],
        [0.0, -height, WATER_PLANE_Z],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0, 2, 1, 0, 3, 2];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Returns the approximate world-space size of the generated map.
pub(crate) fn map_world_size(bounds: MapBounds) -> (f32, f32) {
    let width = f32::sqrt(3.0) * HEX_SIZE * (u32_to_f32(bounds.width()) + 1.0);
    let height = HEX_SIZE * (1.5 * (u32_to_f32(bounds.height()) - 1.0) + 2.0);
    (width, height)
}

/// Returns the center point where a feature model should stand on a tile.
pub(crate) fn tile_center_position(world: &WorldData, bounds: MapBounds, coord: Coord) -> [f32; 3] {
    let mountain_level = mountain_level(world);
    let center = hex_center(coord, bounds);
    let surface = terrain_surface(world, coord, mountain_level);
    let z = if is_mountain(world, coord, mountain_level) {
        surface + MOUNTAIN_PEAK_OFFSET
    } else {
        surface
    };
    [center[0], center[1], z]
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

struct TexturedMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    texture_indices: Vec<u32>,
    indices: Vec<u32>,
}

impl TexturedMeshData {
    fn with_capacity(tile_count: usize) -> Self {
        Self {
            positions: Vec::with_capacity(tile_count * 7),
            normals: Vec::with_capacity(tile_count * 7),
            uvs: Vec::with_capacity(tile_count * 7),
            texture_indices: Vec::with_capacity(tile_count * 7),
            indices: Vec::with_capacity(tile_count * 18),
        }
    }

    fn push_hex(&mut self, coord: Coord, bounds: MapBounds, texture_index: u32) {
        let center = hex_center(coord, bounds);
        let start =
            u32::try_from(self.positions.len()).expect("chunk mesh vertex count fits in u32");

        self.positions.push([center[0], center[1], 0.0]);
        self.normals.push([0.0, 0.0, 1.0]);
        self.uvs.push([0.5, 0.5]);
        self.texture_indices.push(texture_index);

        for angle in CORNER_ANGLES {
            self.positions.push([
                center[0] + HEX_SIZE * angle.cos(),
                center[1] + HEX_SIZE * angle.sin(),
                0.0,
            ]);
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs
                .push([0.5 + 0.5 * angle.cos(), 0.5 - 0.5 * angle.sin()]);
            self.texture_indices.push(texture_index);
        }

        for corner in 0..6 {
            let a = start + 1 + corner;
            let b = start + 1 + ((corner + 1) % 6);
            self.indices.extend_from_slice(&[start, a, b]);
        }
    }

    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(ATTRIBUTE_TEXTURE_INDEX, self.texture_indices);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

struct Terrain3dMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    texture_indices: Vec<u32>,
    indices: Vec<u32>,
}

impl Terrain3dMeshData {
    fn with_capacity(tile_count: usize) -> Self {
        Self {
            positions: Vec::with_capacity(tile_count * 19),
            normals: Vec::with_capacity(tile_count * 19),
            uvs: Vec::with_capacity(tile_count * 19),
            texture_indices: Vec::with_capacity(tile_count * 19),
            indices: Vec::with_capacity(tile_count * 54),
        }
    }

    fn push_top(
        &mut self,
        coord: Coord,
        bounds: MapBounds,
        surface: f32,
        texture_index: u32,
        is_mountain: bool,
    ) {
        let center = hex_center(coord, bounds);
        let center_z = if is_mountain {
            surface + MOUNTAIN_PEAK_OFFSET
        } else {
            surface
        };
        let start =
            u32::try_from(self.positions.len()).expect("chunk mesh vertex count fits in u32");

        self.positions.push([center[0], center[1], center_z]);
        self.normals.push([0.0, 0.0, 1.0]);
        self.uvs.push([0.5, 0.5]);
        self.texture_indices.push(texture_index);

        for angle in CORNER_ANGLES {
            self.positions.push([
                center[0] + HEX_SIZE * angle.cos(),
                center[1] + HEX_SIZE * angle.sin(),
                surface,
            ]);
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs
                .push([0.5 + 0.5 * angle.cos(), 0.5 - 0.5 * angle.sin()]);
            self.texture_indices.push(texture_index);
        }

        for corner in 0..6 {
            let a = start + 1 + corner;
            let b = start + 1 + ((corner + 1) % 6);
            self.indices.extend_from_slice(&[start, a, b]);
        }
    }

    fn push_side(
        &mut self,
        coord: Coord,
        bounds: MapBounds,
        direction: Direction,
        lower: f32,
        upper: f32,
        texture_index: u32,
    ) {
        let [left, right] = edge_corners(direction);
        let center = hex_center(coord, bounds);
        let left_angle = CORNER_ANGLES[left];
        let right_angle = CORNER_ANGLES[right];
        let left_xy = [
            center[0] + HEX_SIZE * left_angle.cos(),
            center[1] + HEX_SIZE * left_angle.sin(),
        ];
        let right_xy = [
            center[0] + HEX_SIZE * right_angle.cos(),
            center[1] + HEX_SIZE * right_angle.sin(),
        ];
        let normal = side_normal(center, left_xy, right_xy);
        let start =
            u32::try_from(self.positions.len()).expect("chunk mesh vertex count fits in u32");

        self.positions.push([left_xy[0], left_xy[1], upper]);
        self.positions.push([right_xy[0], right_xy[1], upper]);
        self.positions.push([right_xy[0], right_xy[1], lower]);
        self.positions.push([left_xy[0], left_xy[1], lower]);
        self.normals.extend_from_slice(&[normal; 4]);
        self.uvs
            .extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        self.texture_indices.extend_from_slice(&[texture_index; 4]);
        self.indices
            .extend_from_slice(&[start, start + 2, start + 1, start, start + 3, start + 2]);
    }

    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(ATTRIBUTE_TEXTURE_INDEX, self.texture_indices);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
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

fn terrain_surface(world: &WorldData, coord: Coord, mountain_level: u8) -> f32 {
    let height_level = *world
        .tectonics
        .height_level
        .get(coord)
        .expect("rendered coord must be in bounds");

    match height_level {
        0 => OCEAN_HEIGHT,
        1 => COAST_HEIGHT,
        level if level >= mountain_level => MOUNTAIN_HEIGHT,
        _ if world
            .tectonics
            .is_hilly
            .get(coord)
            .copied()
            .unwrap_or(false) =>
        {
            HILL_HEIGHT
        }
        _ => LAND_HEIGHT,
    }
}

fn is_mountain(world: &WorldData, coord: Coord, mountain_level: u8) -> bool {
    world
        .tectonics
        .height_level
        .get(coord)
        .copied()
        .is_some_and(|level| level >= mountain_level)
}

fn edge_corners(direction: Direction) -> [usize; 2] {
    match direction {
        Direction::East => [5, 0],
        Direction::NorthEast => [0, 1],
        Direction::NorthWest => [1, 2],
        Direction::West => [2, 3],
        Direction::SouthWest => [3, 4],
        Direction::SouthEast => [4, 5],
    }
}

fn side_normal(center: [f32; 2], left: [f32; 2], right: [f32; 2]) -> [f32; 3] {
    let midpoint = [(left[0] + right[0]) * 0.5, (left[1] + right[1]) * 0.5];
    let x = midpoint[0] - center[0];
    let y = midpoint[1] - center[1];
    let length = (x.mul_add(x, y * y)).sqrt();
    if length == 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [x / length, y / length, 0.0]
    }
}
