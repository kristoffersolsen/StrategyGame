//! Continuous terrain surface prototype code.

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::Mesh,
    render::render_resource::PrimitiveTopology,
};
use coords::{Coord, Direction, MapBounds};

use crate::{
    rendering::{
        hex_mesh::{HEX_SIZE, hex_center},
        terrain_material::{ATTRIBUTE_TERRAIN_BLEND_LAYERS, ATTRIBUTE_TERRAIN_BLEND_WEIGHTS},
    },
    sim::{
        StaticTerrain,
        terrain::{EdgeClass, edge_class},
    },
};

const COAST_HEIGHT: f32 = 0.0;
const OCEAN_DEPTH: f32 = -8.0;
const LAND_STEP: f32 = 4.0;
pub(crate) const CHUNK_SIZE: u32 = 16;
const CASE_SPACING: f32 = 54.0;
const CORNER_RADIUS: f32 = 16.0;
const BRIDGE_WIDTH: f32 = 9.0;
const CORE_RADIUS_RATIO: f32 = 0.72;
const DEFAULT_CORE_SUBDIVISIONS: u8 = 3;
const DEFAULT_BRIDGE_SEGMENTS: u8 = 1;
const DEFAULT_HILL_DISPLACEMENT: f32 = 5.0;
const DEFAULT_MOUNTAIN_DISPLACEMENT: f32 = 15.0;
const DEFAULT_EDGE_TAPER_START: f32 = 0.1;
const DEFAULT_EDGE_TAPER_END: f32 = 1.0;
const DEFAULT_HILL_BASE_WEIGHT: f32 = 0.70;
const DEFAULT_HILL_NOISE_WEIGHT: f32 = 0.25;
const DEFAULT_MOUNTAIN_BASE_WEIGHT: f32 = 0.45;
const DEFAULT_MOUNTAIN_NOISE_WEIGHT: f32 = 0.55;
const DEFAULT_MOUNTAIN_NEIGHBOR_WEIGHT: f32 = 1.0;
const DEFAULT_MOUNTAIN_NEIGHBOR_SCALE_CAP: f32 = 1.20;
const DEFAULT_LOW_NOISE_FREQUENCY: f32 = 0.9;
const DEFAULT_HIGH_NOISE_FREQUENCY: f32 = 0.17;
const DEFAULT_LOW_NOISE_WEIGHT: f32 = 0.65;
const DEFAULT_HIGH_NOISE_WEIGHT: f32 = 0.35;
const DEFAULT_LOW_NOISE_SALT: u32 = 0x58A5_4D31;
const DEFAULT_HIGH_NOISE_SALT: u32 = 0xB3C9_27A1;
const TEXTURE_LAYER_OCEAN: u8 = 0;
const TEXTURE_LAYER_COAST: u8 = 1;
const TEXTURE_LAYER_TROPICAL: u8 = 2;
const TEXTURE_LAYER_TEMPERATE: u8 = 3;
const TEXTURE_LAYER_BOREAL: u8 = 4;
const TEXTURE_LAYER_GRASSLAND: u8 = 5;
const TEXTURE_LAYER_DESERT: u8 = 6;
const TEXTURE_LAYER_TUNDRA: u8 = 7;
const TEXTURE_LAYER_MOUNTAIN: u8 = 8;
const CORNER_ANGLES: [f32; 6] = [
    std::f32::consts::PI / 6.0,
    std::f32::consts::PI / 2.0,
    5.0 * std::f32::consts::PI / 6.0,
    7.0 * std::f32::consts::PI / 6.0,
    3.0 * std::f32::consts::PI / 2.0,
    11.0 * std::f32::consts::PI / 6.0,
];
const GEOMETRY_DIRECTIONS: [Direction; 6] = [
    Direction::East,
    Direction::NorthEast,
    Direction::NorthWest,
    Direction::West,
    Direction::SouthWest,
    Direction::SouthEast,
];

/// Terrain mesh construction parameters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainMeshParams {
    pub(crate) core_radius: f32,
    pub(crate) core_subdivisions: u8,
    pub(crate) bridge_segments: u8,
    pub(crate) displacement: TerrainDisplacementParams,
}

impl Default for TerrainMeshParams {
    fn default() -> Self {
        Self {
            core_radius: HEX_SIZE * CORE_RADIUS_RATIO,
            core_subdivisions: DEFAULT_CORE_SUBDIVISIONS,
            bridge_segments: DEFAULT_BRIDGE_SEGMENTS,
            displacement: TerrainDisplacementParams::default(),
        }
    }
}

/// One value-noise octave used for core hill and mountain displacement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainNoiseLayer {
    pub(crate) frequency: f32,
    pub(crate) weight: f32,
    pub(crate) salt: u32,
}

/// Tunable parameters for hill and mountain core displacement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainDisplacementParams {
    pub(crate) hill_amplitude: f32,
    pub(crate) mountain_amplitude: f32,
    pub(crate) edge_taper_start: f32,
    pub(crate) edge_taper_end: f32,
    pub(crate) hill_base_weight: f32,
    pub(crate) hill_noise_weight: f32,
    pub(crate) mountain_base_weight: f32,
    pub(crate) mountain_noise_weight: f32,
    pub(crate) mountain_neighbor_weight: f32,
    pub(crate) mountain_neighbor_scale_cap: f32,
    pub(crate) noise_layers: [TerrainNoiseLayer; 2],
}

impl Default for TerrainDisplacementParams {
    fn default() -> Self {
        Self {
            hill_amplitude: DEFAULT_HILL_DISPLACEMENT,
            mountain_amplitude: DEFAULT_MOUNTAIN_DISPLACEMENT,
            edge_taper_start: DEFAULT_EDGE_TAPER_START,
            edge_taper_end: DEFAULT_EDGE_TAPER_END,
            hill_base_weight: DEFAULT_HILL_BASE_WEIGHT,
            hill_noise_weight: DEFAULT_HILL_NOISE_WEIGHT,
            mountain_base_weight: DEFAULT_MOUNTAIN_BASE_WEIGHT,
            mountain_noise_weight: DEFAULT_MOUNTAIN_NOISE_WEIGHT,
            mountain_neighbor_weight: DEFAULT_MOUNTAIN_NEIGHBOR_WEIGHT,
            mountain_neighbor_scale_cap: DEFAULT_MOUNTAIN_NEIGHBOR_SCALE_CAP,
            noise_layers: [
                TerrainNoiseLayer {
                    frequency: DEFAULT_LOW_NOISE_FREQUENCY,
                    weight: DEFAULT_LOW_NOISE_WEIGHT,
                    salt: DEFAULT_LOW_NOISE_SALT,
                },
                TerrainNoiseLayer {
                    frequency: DEFAULT_HIGH_NOISE_FREQUENCY,
                    weight: DEFAULT_HIGH_NOISE_WEIGHT,
                    salt: DEFAULT_HIGH_NOISE_SALT,
                },
            ],
        }
    }
}

/// Prototype edge class used by the isolated R1 corner harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrototypeEdgeClass {
    /// Same tier.
    Flat,
    /// One-tier passable transition.
    Slope,
    /// Multi-tier or forced impassable transition.
    Cliff,
}

/// One synthetic three-tile corner case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerPrototypeCase {
    /// Human-readable case name.
    pub name: &'static str,
    /// Clockwise tile tiers around the corner.
    pub levels: [u8; 3],
    /// Edges forced to cliff independent of tier delta.
    pub forced_cliff_edges: [bool; 3],
}

/// CPU mesh data for an R1 prototype mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct PrototypeMeshData {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals.
    pub normals: Vec<[f32; 3]>,
    /// Vertex colors.
    pub colors: Vec<[f32; 4]>,
    /// Up to four texture-array layer indices per vertex.
    pub texture_layers: Vec<[f32; 4]>,
    /// Blend weights for `texture_layers`.
    pub blend_weights: Vec<[f32; 4]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

impl PrototypeMeshData {
    /// Converts the prototype data into a Bevy mesh.
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_attribute(ATTRIBUTE_TERRAIN_BLEND_LAYERS, self.texture_layers);
        mesh.insert_attribute(ATTRIBUTE_TERRAIN_BLEND_WEIGHTS, self.blend_weights);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// World-space height for a discrete terrain tier.
pub fn tier_height(height_level: u8, mountain_level: u8) -> f32 {
    match height_level {
        0 => OCEAN_DEPTH,
        1 => COAST_HEIGHT,
        level => LAND_STEP * f32::from(level.min(mountain_level).saturating_sub(1)),
    }
}

/// Builds a continuous-surface terrain mesh for one chunk.
pub(crate) fn build_terrain_chunk_mesh(
    chunk_origin: Coord,
    terrain: &StaticTerrain,
    params: TerrainMeshParams,
) -> Mesh {
    build_terrain_chunk_data(chunk_origin, terrain, params).into_mesh()
}

fn build_terrain_chunk_data(
    chunk_origin: Coord,
    terrain: &StaticTerrain,
    params: TerrainMeshParams,
) -> PrototypeMeshData {
    let bounds = terrain.bounds;
    let max_q = (chunk_origin.q + CHUNK_SIZE.cast_signed()).min(bounds.width().cast_signed());
    let max_r = (chunk_origin.r + CHUNK_SIZE.cast_signed()).min(bounds.height().cast_signed());
    let tile_count = usize::try_from((max_q - chunk_origin.q) * (max_r - chunk_origin.r))
        .expect("chunk tile count must be non-negative");

    let mut data = PrototypeMeshData {
        positions: Vec::with_capacity(tile_count * 96),
        normals: Vec::with_capacity(tile_count * 96),
        colors: Vec::with_capacity(tile_count * 96),
        texture_layers: Vec::with_capacity(tile_count * 96),
        blend_weights: Vec::with_capacity(tile_count * 96),
        indices: Vec::with_capacity(tile_count * 180),
    };

    for r in chunk_origin.r..max_r {
        for q in chunk_origin.q..max_q {
            let coord = bounds
                .normalize(Coord::new(q, r))
                .expect("chunk iteration keeps r in bounds");
            push_terrain_tile(&mut data, coord, terrain, params);
        }
    }

    data
}

fn push_terrain_tile(
    data: &mut PrototypeMeshData,
    coord: Coord,
    terrain: &StaticTerrain,
    params: TerrainMeshParams,
) {
    push_core(data, coord, terrain, params);

    for direction in GEOMETRY_DIRECTIONS {
        let Some(neighbor) = terrain.bounds.neighbor(coord, direction) else {
            continue;
        };
        if edge_owner(coord, neighbor) == coord {
            push_terrain_bridge(data, coord, neighbor, direction, terrain, params);
        }
    }

    for corner in 0..6 {
        let Some(corner_tiles) = corner_tiles(coord, corner, terrain.bounds) else {
            continue;
        };
        if corner_owner(corner_tiles) == coord {
            push_corner_stitch(data, coord, corner, corner_tiles, terrain, params);
        }
    }
}

fn push_core(
    data: &mut PrototypeMeshData,
    coord: Coord,
    terrain: &StaticTerrain,
    params: TerrainMeshParams,
) {
    let center_xy = hex_center(coord, terrain.bounds);
    let base_height = terrain_height(terrain, coord);
    let color = terrain_color(terrain, coord);
    let blend = tile_texture_blend(terrain, coord);
    let subdivisions = u32::from(params.core_subdivisions.max(1));
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut texture_layers = Vec::new();
    let mut blend_weights = Vec::new();
    let mut indices = Vec::new();

    for corner in 0..6 {
        push_core_sector(
            &mut positions,
            &mut colors,
            &mut texture_layers,
            &mut blend_weights,
            &mut indices,
            CoreSector {
                coord,
                terrain,
                center: center_xy,
                base_height,
                radius: params.core_radius,
                displacement: params.displacement,
                color,
                blend,
                corner,
                subdivisions,
            },
        );
    }

    append_smooth_geometry(
        data,
        positions,
        colors,
        texture_layers,
        blend_weights,
        indices,
    );
}

fn push_terrain_bridge(
    data: &mut PrototypeMeshData,
    coord: Coord,
    neighbor: Coord,
    direction: Direction,
    terrain: &StaticTerrain,
    params: TerrainMeshParams,
) {
    let [left_corner, right_corner] = edge_corners(direction);
    let center = hex_center(coord, terrain.bounds);
    let neighbor_center = offset_center(center, direction);
    let current_color = terrain_color(terrain, coord);
    let neighbor_color = terrain_color(terrain, neighbor);
    let left_outer = outer_corner(center, left_corner);
    let right_outer = outer_corner(center, right_corner);
    let current_left = point_toward_xy(center, left_outer, params.core_radius);
    let current_right = point_toward_xy(center, right_outer, params.core_radius);
    let neighbor_left = point_toward_xy(neighbor_center, left_outer, params.core_radius);
    let neighbor_right = point_toward_xy(neighbor_center, right_outer, params.core_radius);
    let class = edge_class(coord, neighbor, terrain);
    let color = match class {
        EdgeClass::Flat => mix_color(current_color, neighbor_color, 0.5),
        EdgeClass::Slope => [0.68, 0.56, 0.34, 1.0],
        EdgeClass::Cliff => [0.40, 0.38, 0.36, 1.0],
    };

    push_displaced_bridge_grid(
        data,
        DisplacedBridgeGrid {
            terrain,
            current: coord,
            neighbor,
            current_center: center,
            neighbor_center,
            current_left,
            current_right,
            neighbor_left,
            neighbor_right,
            radius: params.core_radius,
            displacement: params.displacement,
            side_segments: params.core_subdivisions,
            cross_segments: params.bridge_segments,
            color,
        },
    );
}

#[derive(Clone, Copy)]
struct DisplacedBridgeGrid<'a> {
    terrain: &'a StaticTerrain,
    current: Coord,
    neighbor: Coord,
    current_center: [f32; 2],
    neighbor_center: [f32; 2],
    current_left: [f32; 2],
    current_right: [f32; 2],
    neighbor_left: [f32; 2],
    neighbor_right: [f32; 2],
    radius: f32,
    displacement: TerrainDisplacementParams,
    side_segments: u8,
    cross_segments: u8,
    color: [f32; 4],
}

fn push_displaced_bridge_grid(data: &mut PrototypeMeshData, grid: DisplacedBridgeGrid) {
    let side_segments = u32::from(grid.side_segments.max(1));
    let cross_segments = u32::from(grid.cross_segments.max(1));
    let vertex_columns = cross_segments + 1;
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut texture_layers = Vec::new();
    let mut blend_weights = Vec::new();
    let mut indices = Vec::new();

    for side in 0..=side_segments {
        let side_t = u32_to_f32(side) / u32_to_f32(side_segments);
        let current_edge = lerp2(grid.current_left, grid.current_right, side_t);
        let neighbor_edge = lerp2(grid.neighbor_left, grid.neighbor_right, side_t);
        let current_height = displaced_core_height_at(
            grid.terrain,
            grid.current,
            grid.current_center,
            grid.radius,
            grid.displacement,
            current_edge,
        );
        let neighbor_height = displaced_core_height_at(
            grid.terrain,
            grid.neighbor,
            grid.neighbor_center,
            grid.radius,
            grid.displacement,
            neighbor_edge,
        );

        for cross in 0..=cross_segments {
            let cross_t = u32_to_f32(cross) / u32_to_f32(cross_segments);
            let xy = lerp2(current_edge, neighbor_edge, cross_t);
            positions.push([xy[0], xy[1], lerp(current_height, neighbor_height, cross_t)]);
            colors.push(grid.color);
            let blend = tile_pair_blend(grid.terrain, grid.current, grid.neighbor, cross_t);
            texture_layers.push(blend.layers);
            blend_weights.push(blend.weights);
        }
    }

    for side in 0..side_segments {
        for cross in 0..cross_segments {
            let v0 = side * vertex_columns + cross;
            let v1 = (side + 1) * vertex_columns + cross;
            let v2 = (side + 1) * vertex_columns + cross + 1;
            let v3 = side * vertex_columns + cross + 1;
            indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
        }
    }

    append_smooth_geometry(
        data,
        positions,
        colors,
        texture_layers,
        blend_weights,
        indices,
    );
}

fn push_corner_stitch(
    data: &mut PrototypeMeshData,
    coord: Coord,
    corner: usize,
    tiles: [Coord; 3],
    terrain: &StaticTerrain,
    params: TerrainMeshParams,
) {
    let center = hex_center(coord, terrain.bounds);
    let corner_point = outer_corner(center, corner);
    let neighbor_dirs = corner_neighbor_dirs(corner);
    let centers = [
        center,
        offset_center(center, neighbor_dirs[0]),
        offset_center(center, neighbor_dirs[1]),
    ];
    let vertices = [
        displaced_core_position_toward(
            terrain,
            tiles[0],
            centers[0],
            corner_point,
            params.core_radius,
            params.displacement,
        ),
        displaced_core_position_toward(
            terrain,
            tiles[1],
            centers[1],
            corner_point,
            params.core_radius,
            params.displacement,
        ),
        displaced_core_position_toward(
            terrain,
            tiles[2],
            centers[2],
            corner_point,
            params.core_radius,
            params.displacement,
        ),
    ];
    let color = corner_color(terrain, tiles);
    let blend = weighted_tile_blend(
        terrain,
        &[(tiles[0], 1.0), (tiles[1], 1.0), (tiles[2], 1.0)],
    );
    push_triangle_with_blends(data, vertices, [color; 3], [blend; 3]);
}

fn displaced_core_position_toward(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    target: [f32; 2],
    radius: f32,
    displacement: TerrainDisplacementParams,
) -> [f32; 3] {
    let xy = point_toward_xy(center, target, radius);
    [
        xy[0],
        xy[1],
        displaced_core_height_at(terrain, coord, center, radius, displacement, xy),
    ]
}

fn displaced_core_height_at(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    radius: f32,
    displacement: TerrainDisplacementParams,
    xy: [f32; 2],
) -> f32 {
    let base_height = terrain_height(terrain, coord);
    displaced_core_height(
        terrain,
        coord,
        center,
        base_height,
        radius,
        displacement,
        [xy[0], xy[1], base_height],
    )
}

/// Builds the synthetic case suite used by the R1 corner prototype harness.
pub fn corner_prototype_cases() -> Vec<CornerPrototypeCase> {
    let mut cases = vec![
        CornerPrototypeCase {
            name: "equal",
            levels: [2, 2, 2],
            forced_cliff_edges: [false; 3],
        },
        CornerPrototypeCase {
            name: "one-step-a",
            levels: [2, 3, 2],
            forced_cliff_edges: [false; 3],
        },
        CornerPrototypeCase {
            name: "one-step-b",
            levels: [3, 2, 3],
            forced_cliff_edges: [false; 3],
        },
        CornerPrototypeCase {
            name: "multi-step",
            levels: [2, 4, 3],
            forced_cliff_edges: [false; 3],
        },
    ];

    for edge in 0..3 {
        let mut forced = [false; 3];
        forced[edge] = true;
        cases.push(CornerPrototypeCase {
            name: "forced-cliff",
            levels: [2, 3, 2],
            forced_cliff_edges: forced,
        });
    }

    for a in 2..=4 {
        for b in 2..=4 {
            for c in 2..=4 {
                cases.push(CornerPrototypeCase {
                    name: "exhaustive",
                    levels: [a, b, c],
                    forced_cliff_edges: [false; 3],
                });
            }
        }
    }

    cases
}

/// Classifies one prototype edge.
pub fn prototype_edge_class(
    left_level: u8,
    right_level: u8,
    forced_cliff: bool,
) -> PrototypeEdgeClass {
    if forced_cliff || left_level.abs_diff(right_level) >= 2 {
        PrototypeEdgeClass::Cliff
    } else if left_level == right_level {
        PrototypeEdgeClass::Flat
    } else {
        PrototypeEdgeClass::Slope
    }
}

/// Builds CPU mesh data for one isolated three-tier corner case.
pub fn build_three_tier_corner_prototype_data(case: CornerPrototypeCase) -> PrototypeMeshData {
    let mut data = PrototypeMeshData {
        positions: Vec::new(),
        normals: Vec::new(),
        colors: Vec::new(),
        texture_layers: Vec::new(),
        blend_weights: Vec::new(),
        indices: Vec::new(),
    };
    push_case(&mut data, case, [0.0, 0.0]);
    data
}

/// Builds CPU mesh data for every R1 case, laid out in a grid.
pub fn build_corner_prototype_suite_data() -> PrototypeMeshData {
    let cases = corner_prototype_cases();
    let columns = 7;
    let mut data = PrototypeMeshData {
        positions: Vec::new(),
        normals: Vec::new(),
        colors: Vec::new(),
        texture_layers: Vec::new(),
        blend_weights: Vec::new(),
        indices: Vec::new(),
    };

    for (index, case) in cases.into_iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        push_case(
            &mut data,
            case,
            [
                usize_to_f32(column) * CASE_SPACING,
                -usize_to_f32(row) * CASE_SPACING,
            ],
        );
    }

    data
}

fn terrain_height(terrain: &StaticTerrain, coord: Coord) -> f32 {
    let level = terrain.height_level.get(coord).copied().unwrap_or(0);
    tier_height(level, terrain.mountain_level)
}

#[derive(Clone, Copy)]
struct TerrainBlend {
    layers: [f32; 4],
    weights: [f32; 4],
}

fn tile_texture_blend(terrain: &StaticTerrain, coord: Coord) -> TerrainBlend {
    solid_texture_blend(tile_texture_layer(terrain, coord))
}

fn tile_pair_blend(
    terrain: &StaticTerrain,
    current: Coord,
    neighbor: Coord,
    t: f32,
) -> TerrainBlend {
    let current_layer = tile_texture_layer(terrain, current);
    let neighbor_layer = tile_texture_layer(terrain, neighbor);
    if current_layer == neighbor_layer {
        return solid_texture_blend(current_layer);
    }

    TerrainBlend {
        layers: [
            f32::from(current_layer),
            f32::from(neighbor_layer),
            0.0,
            0.0,
        ],
        weights: [1.0 - t, t, 0.0, 0.0],
    }
}

fn solid_texture_blend(layer: u8) -> TerrainBlend {
    TerrainBlend {
        layers: [f32::from(layer), 0.0, 0.0, 0.0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }
}

fn weighted_tile_blend(terrain: &StaticTerrain, tiles: &[(Coord, f32)]) -> TerrainBlend {
    let mut layers = [0_u8; 4];
    let mut weights = [0.0; 4];
    let mut layer_count = 0;

    for (coord, weight) in tiles {
        if *weight <= f32::EPSILON {
            continue;
        }

        let layer = tile_texture_layer(terrain, *coord);
        if let Some(index) = layers[..layer_count]
            .iter()
            .position(|candidate| *candidate == layer)
        {
            weights[index] += *weight;
        } else {
            debug_assert!(layer_count < layers.len());
            layers[layer_count] = layer;
            weights[layer_count] = *weight;
            layer_count += 1;
        }
    }

    let total_weight: f32 = weights.iter().sum();
    if total_weight <= f32::EPSILON {
        return solid_texture_blend(TEXTURE_LAYER_GRASSLAND);
    }

    TerrainBlend {
        layers: layers.map(f32::from),
        weights: weights.map(|weight| weight / total_weight),
    }
}

fn tile_texture_layer(terrain: &StaticTerrain, coord: Coord) -> u8 {
    let level = terrain.height_level.get(coord).copied().unwrap_or(0);
    match level {
        0 => TEXTURE_LAYER_OCEAN,
        1 => TEXTURE_LAYER_COAST,
        level if level >= terrain.mountain_level => TEXTURE_LAYER_MOUNTAIN,
        _ => terrain
            .biome
            .get(coord)
            .copied()
            .flatten()
            .map_or(TEXTURE_LAYER_GRASSLAND, |handle| {
                biome_texture_layer(terrain.biome_registry.id_of(handle))
            }),
    }
}

fn biome_texture_layer(id: &str) -> u8 {
    match id {
        "tropical_rainforest" => TEXTURE_LAYER_TROPICAL,
        "temperate_forest" => TEXTURE_LAYER_TEMPERATE,
        "boreal_forest" => TEXTURE_LAYER_BOREAL,
        "desert" => TEXTURE_LAYER_DESERT,
        "tundra" => TEXTURE_LAYER_TUNDRA,
        _ => TEXTURE_LAYER_GRASSLAND,
    }
}

#[derive(Clone, Copy)]
struct CoreSector<'a> {
    coord: Coord,
    terrain: &'a StaticTerrain,
    center: [f32; 2],
    base_height: f32,
    radius: f32,
    displacement: TerrainDisplacementParams,
    color: [f32; 4],
    blend: TerrainBlend,
    corner: usize,
    subdivisions: u32,
}

fn push_core_sector(
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    texture_layers: &mut Vec<[f32; 4]>,
    blend_weights: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    sector: CoreSector,
) {
    let start = u32::try_from(positions.len()).expect("core vertex count fits in u32");
    let a = inner_corner(
        sector.center,
        sector.corner,
        sector.radius,
        sector.base_height,
    );
    let b = inner_corner(
        sector.center,
        (sector.corner + 1) % 6,
        sector.radius,
        sector.base_height,
    );
    let center = [sector.center[0], sector.center[1], sector.base_height];

    for i in 0..=sector.subdivisions {
        for j in 0..=sector.subdivisions - i {
            let position = core_sector_position(center, a, b, i, j, sector.subdivisions);
            let displaced = [
                position[0],
                position[1],
                displaced_core_height(
                    sector.terrain,
                    sector.coord,
                    sector.center,
                    sector.base_height,
                    sector.radius,
                    sector.displacement,
                    position,
                ),
            ];
            positions.push(displaced);
            colors.push(sector.color);
            texture_layers.push(sector.blend.layers);
            blend_weights.push(sector.blend.weights);
        }
    }

    for i in 0..sector.subdivisions {
        for j in 0..sector.subdivisions - i {
            let v0 = start + core_grid_index(i, j, sector.subdivisions);
            let v1 = start + core_grid_index(i + 1, j, sector.subdivisions);
            let v2 = start + core_grid_index(i, j + 1, sector.subdivisions);
            indices.extend_from_slice(&[v0, v1, v2]);

            if j < sector.subdivisions - i - 1 {
                let v3 = start + core_grid_index(i + 1, j + 1, sector.subdivisions);
                indices.extend_from_slice(&[v1, v3, v2]);
            }
        }
    }
}

fn core_sector_position(
    center: [f32; 3],
    corner_a: [f32; 3],
    corner_b: [f32; 3],
    row: u32,
    column: u32,
    subdivisions: u32,
) -> [f32; 3] {
    let row = u32_to_f32(row);
    let column = u32_to_f32(column);
    let total = u32_to_f32(subdivisions);
    let center_weight = (total - row - column) / total;
    let a_weight = row / total;
    let b_weight = column / total;
    [
        center[0] * center_weight + corner_a[0] * a_weight + corner_b[0] * b_weight,
        center[1] * center_weight + corner_a[1] * a_weight + corner_b[1] * b_weight,
        center[2] * center_weight + corner_a[2] * a_weight + corner_b[2] * b_weight,
    ]
}

fn core_grid_index(i: u32, j: u32, subdivisions: u32) -> u32 {
    let previous_rows = i * (subdivisions + 1) - i.saturating_sub(1) * i / 2;
    previous_rows + j
}

fn displaced_core_height(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    base_height: f32,
    radius: f32,
    displacement: TerrainDisplacementParams,
    position: [f32; 3],
) -> f32 {
    let amplitude = displacement_amplitude(terrain, coord, displacement);
    if amplitude == 0.0 {
        return base_height;
    }

    let dx = position[0] - center[0];
    let dy = position[1] - center[1];
    let radial = (dx.mul_add(dx, dy * dy)).sqrt() / radius;
    let edge_progress = hex_edge_progress([dx, dy], radius);
    let edge_taper = 1.0
        - smoothstep(
            displacement.edge_taper_start,
            displacement.edge_taper_end,
            edge_progress,
        );
    let edge_support =
        displacement_edge_support(terrain, coord, center, radius, position, displacement);
    let edge_taper = edge_taper + (1.0 - edge_taper) * edge_support;
    if edge_taper <= 0.0 {
        return base_height;
    }

    let shape = 1.0 - radial.clamp(0.0, 1.0).powi(2);
    let noise = displacement_noise(position[0], position[1], displacement);
    let level = terrain.height_level.get(coord).copied().unwrap_or(0);
    let vertical_offset = if level >= terrain.mountain_level {
        amplitude
            * edge_taper
            * shape
            * (displacement.mountain_base_weight + displacement.mountain_noise_weight * noise.abs())
    } else {
        amplitude
            * edge_taper
            * shape
            * (displacement.hill_base_weight + displacement.hill_noise_weight * noise)
    };

    base_height + vertical_offset.max(0.0)
}

fn displacement_amplitude(
    terrain: &StaticTerrain,
    coord: Coord,
    displacement: TerrainDisplacementParams,
) -> f32 {
    let base_amplitude = base_displacement_amplitude(terrain, coord, displacement);
    if is_mountain_tile(terrain, coord) {
        base_amplitude * mountain_neighbor_scale(terrain, coord, displacement)
    } else {
        base_amplitude
    }
}

fn base_displacement_amplitude(
    terrain: &StaticTerrain,
    coord: Coord,
    displacement: TerrainDisplacementParams,
) -> f32 {
    let level = terrain.height_level.get(coord).copied().unwrap_or(0);
    if level >= terrain.mountain_level {
        displacement.mountain_amplitude
    } else if terrain.is_hilly.get(coord).copied().unwrap_or(false) {
        displacement.hill_amplitude
    } else {
        0.0
    }
}

fn is_mountain_tile(terrain: &StaticTerrain, coord: Coord) -> bool {
    terrain.height_level.get(coord).copied().unwrap_or(0) >= terrain.mountain_level
}

fn mountain_neighbor_scale(
    terrain: &StaticTerrain,
    coord: Coord,
    displacement: TerrainDisplacementParams,
) -> f32 {
    let mountain_ratio = mountain_neighbor_ratio(terrain, coord);
    let scale = 1.0 + mountain_ratio * displacement.mountain_neighbor_weight;
    scale.clamp(0.0, displacement.mountain_neighbor_scale_cap.max(0.0))
}

fn mountain_neighbor_ratio(terrain: &StaticTerrain, coord: Coord) -> f32 {
    let mut mountain_neighbors = 0.0;
    let mut total_neighbors = 0.0;

    for direction in GEOMETRY_DIRECTIONS {
        let Some(neighbor) = terrain.bounds.neighbor(coord, direction) else {
            continue;
        };
        total_neighbors += 1.0;
        if is_mountain_tile(terrain, neighbor) {
            mountain_neighbors += 1.0;
        }
    }

    if total_neighbors <= f32::EPSILON {
        0.0
    } else {
        mountain_neighbors / total_neighbors
    }
}

fn displacement_edge_support(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    radius: f32,
    position: [f32; 3],
    displacement: TerrainDisplacementParams,
) -> f32 {
    let current_amplitude = base_displacement_amplitude(terrain, coord, displacement);
    if current_amplitude <= f32::EPSILON {
        return 0.0;
    }

    let offset = [position[0] - center[0], position[1] - center[1]];
    let apothem = hex_apothem(radius);
    let mut total_weight = 0.0;
    let mut total_support = 0.0;

    for direction in GEOMETRY_DIRECTIONS {
        let side_progress = dot2(offset, direction_unit(direction)) / apothem;
        let side_weight = side_progress.max(0.0).powi(4);
        if side_weight <= f32::EPSILON {
            continue;
        }

        total_weight += side_weight;
        total_support += side_weight
            * terrain
                .bounds
                .neighbor(coord, direction)
                .map_or(0.0, |neighbor| {
                    let neighbor_amplitude =
                        base_displacement_amplitude(terrain, neighbor, displacement);
                    (neighbor_amplitude / current_amplitude).clamp(0.0, 1.0)
                });
    }

    if total_weight <= f32::EPSILON {
        0.0
    } else {
        total_support / total_weight
    }
}

fn hex_edge_progress(offset: [f32; 2], radius: f32) -> f32 {
    let apothem = hex_apothem(radius);
    GEOMETRY_DIRECTIONS
        .into_iter()
        .map(|direction| dot2(offset, direction_unit(direction)) / apothem)
        .fold(0.0, f32::max)
        .clamp(0.0, 1.0)
}

fn hex_apothem(radius: f32) -> f32 {
    radius * (std::f32::consts::PI / 6.0).cos()
}

fn direction_unit(direction: Direction) -> [f32; 2] {
    let sqrt_3 = f32::sqrt(3.0);
    match direction {
        Direction::East => [1.0, 0.0],
        Direction::West => [-1.0, 0.0],
        Direction::NorthEast => [0.5, 0.5 * sqrt_3],
        Direction::NorthWest => [-0.5, 0.5 * sqrt_3],
        Direction::SouthEast => [0.5, -0.5 * sqrt_3],
        Direction::SouthWest => [-0.5, -0.5 * sqrt_3],
    }
}

fn dot2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0].mul_add(b[0], a[1] * b[1])
}

fn displacement_noise(x: f32, y: f32, displacement: TerrainDisplacementParams) -> f32 {
    let mut total_weight = 0.0;
    let mut sample = 0.0;

    for layer in displacement.noise_layers {
        sample += value_noise(x * layer.frequency, y * layer.frequency, layer.salt) * layer.weight;
        total_weight += layer.weight;
    }

    if total_weight <= f32::EPSILON {
        0.0
    } else {
        (sample / total_weight).clamp(-1.0, 1.0)
    }
}

fn value_noise(sample_x: f32, sample_y: f32, salt: u32) -> f32 {
    let grid_x = floor_to_i32(sample_x);
    let grid_y = floor_to_i32(sample_y);
    let local_x = sample_x - i32_to_f32(grid_x);
    let local_y = sample_y - i32_to_f32(grid_y);
    let smooth_x = smoothstep(0.0, 1.0, local_x);
    let smooth_y = smoothstep(0.0, 1.0, local_y);
    let northwest = hash_unit(grid_x, grid_y, salt);
    let northeast = hash_unit(grid_x + 1, grid_y, salt);
    let southwest = hash_unit(grid_x, grid_y + 1, salt);
    let southeast = hash_unit(grid_x + 1, grid_y + 1, salt);
    let north = lerp(northwest, northeast, smooth_x);
    let south = lerp(southwest, southeast, smooth_x);
    lerp(north, south, smooth_y) * 2.0 - 1.0
}

fn hash_unit(x: i32, y: i32, salt: u32) -> f32 {
    let mut value = u32::from_ne_bytes(x.to_ne_bytes()).wrapping_mul(0x9E37_79B1);
    value ^= u32::from_ne_bytes(y.to_ne_bytes()).wrapping_mul(0x85EB_CA77);
    value ^= salt;
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^= value >> 16;
    u32_to_f32(value) / u32_to_f32(u32::MAX)
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a.mul_add(1.0 - t, b * t)
}

#[allow(clippy::cast_possible_truncation)]
fn floor_to_i32(value: f32) -> i32 {
    value.floor() as i32
}

fn edge_owner(a: Coord, b: Coord) -> Coord {
    if coord_key(a) <= coord_key(b) { a } else { b }
}

fn corner_owner(coords: [Coord; 3]) -> Coord {
    coords
        .into_iter()
        .min_by_key(|coord| coord_key(*coord))
        .expect("corner always has three coordinates")
}

fn coord_key(coord: Coord) -> (i32, i32) {
    (coord.r, coord.q)
}

fn corner_tiles(coord: Coord, corner: usize, bounds: MapBounds) -> Option<[Coord; 3]> {
    let dirs = corner_neighbor_dirs(corner);
    Some([
        coord,
        bounds.neighbor(coord, dirs[0])?,
        bounds.neighbor(coord, dirs[1])?,
    ])
}

fn corner_neighbor_dirs(corner: usize) -> [Direction; 2] {
    match corner {
        0 => [Direction::East, Direction::NorthEast],
        1 => [Direction::NorthEast, Direction::NorthWest],
        2 => [Direction::NorthWest, Direction::West],
        3 => [Direction::West, Direction::SouthWest],
        4 => [Direction::SouthWest, Direction::SouthEast],
        5 => [Direction::SouthEast, Direction::East],
        _ => unreachable!("hex corner index must be in 0..6"),
    }
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

fn inner_corner(center: [f32; 2], corner: usize, radius: f32, height: f32) -> [f32; 3] {
    let angle = CORNER_ANGLES[corner];
    [
        center[0] + radius * angle.cos(),
        center[1] + radius * angle.sin(),
        height,
    ]
}

fn outer_corner(center: [f32; 2], corner: usize) -> [f32; 2] {
    let angle = CORNER_ANGLES[corner];
    [
        center[0] + HEX_SIZE * angle.cos(),
        center[1] + HEX_SIZE * angle.sin(),
    ]
}

fn offset_center(center: [f32; 2], direction: Direction) -> [f32; 2] {
    let sqrt_3 = f32::sqrt(3.0);
    match direction {
        Direction::East => [center[0] + sqrt_3 * HEX_SIZE, center[1]],
        Direction::West => [center[0] - sqrt_3 * HEX_SIZE, center[1]],
        Direction::NorthEast => [
            center[0] + sqrt_3 * HEX_SIZE * 0.5,
            center[1] + 1.5 * HEX_SIZE,
        ],
        Direction::NorthWest => [
            center[0] - sqrt_3 * HEX_SIZE * 0.5,
            center[1] + 1.5 * HEX_SIZE,
        ],
        Direction::SouthEast => [
            center[0] + sqrt_3 * HEX_SIZE * 0.5,
            center[1] - 1.5 * HEX_SIZE,
        ],
        Direction::SouthWest => [
            center[0] - sqrt_3 * HEX_SIZE * 0.5,
            center[1] - 1.5 * HEX_SIZE,
        ],
    }
}

fn point_toward_xy(center: [f32; 2], target: [f32; 2], radius: f32) -> [f32; 2] {
    let direction = normalize([target[0] - center[0], target[1] - center[1]]);
    [
        center[0] + direction[0] * radius,
        center[1] + direction[1] * radius,
    ]
}

fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [
        a[0].mul_add(1.0 - t, b[0] * t),
        a[1].mul_add(1.0 - t, b[1] * t),
    ]
}

fn terrain_color(terrain: &StaticTerrain, coord: Coord) -> [f32; 4] {
    let level = terrain.height_level.get(coord).copied().unwrap_or(0);
    match level {
        0 => [0.10, 0.23, 0.36, 1.0],
        1 => [0.18, 0.43, 0.65, 1.0],
        level if level >= terrain.mountain_level => [0.44, 0.44, 0.44, 1.0],
        _ => terrain
            .biome
            .get(coord)
            .copied()
            .flatten()
            .map_or([0.48, 0.60, 0.23, 1.0], |handle| {
                biome_color(terrain.biome_registry.id_of(handle))
            }),
    }
}

fn biome_color(id: &str) -> [f32; 4] {
    match id {
        "tropical_rainforest" => [0.18, 0.42, 0.18, 1.0],
        "temperate_forest" => [0.29, 0.49, 0.29, 1.0],
        "boreal_forest" => [0.29, 0.42, 0.33, 1.0],
        "desert" => [0.78, 0.66, 0.43, 1.0],
        "tundra" => [0.54, 0.60, 0.54, 1.0],
        _ => [0.48, 0.60, 0.23, 1.0],
    }
}

fn corner_color(terrain: &StaticTerrain, tiles: [Coord; 3]) -> [f32; 4] {
    let first = terrain_color(terrain, tiles[0]);
    let second = terrain_color(terrain, tiles[1]);
    let third = terrain_color(terrain, tiles[2]);
    [
        (first[0] + second[0] + third[0]) / 3.0,
        (first[1] + second[1] + third[1]) / 3.0,
        (first[2] + second[2] + third[2]) / 3.0,
        1.0,
    ]
}

fn mix_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0].mul_add(1.0 - t, b[0] * t),
        a[1].mul_add(1.0 - t, b[1] * t),
        a[2].mul_add(1.0 - t, b[2] * t),
        1.0,
    ]
}

fn push_case(data: &mut PrototypeMeshData, case: CornerPrototypeCase, offset: [f32; 2]) {
    let points = [
        [offset[0], offset[1] + CORNER_RADIUS],
        [
            offset[0] - CORNER_RADIUS * 0.866_025_4,
            offset[1] - CORNER_RADIUS * 0.5,
        ],
        [
            offset[0] + CORNER_RADIUS * 0.866_025_4,
            offset[1] - CORNER_RADIUS * 0.5,
        ],
    ];
    let center = [offset[0], offset[1]];
    let heights = case.levels.map(|level| tier_height(level, 5));

    push_triangle(
        data,
        [
            [points[0][0], points[0][1], heights[0]],
            [points[1][0], points[1][1], heights[1]],
            [points[2][0], points[2][1], heights[2]],
        ],
        color_for_class(corner_dominant_class(case)),
    );

    for edge in 0..3 {
        let next = (edge + 1) % 3;
        let class = prototype_edge_class(
            case.levels[edge],
            case.levels[next],
            case.forced_cliff_edges[edge],
        );
        push_bridge(
            data,
            center,
            points[edge],
            points[next],
            heights[edge],
            heights[next],
            class,
        );
    }
}

fn push_bridge(
    data: &mut PrototypeMeshData,
    center: [f32; 2],
    left: [f32; 2],
    right: [f32; 2],
    left_height: f32,
    right_height: f32,
    class: PrototypeEdgeClass,
) {
    let midpoint = [(left[0] + right[0]) * 0.5, (left[1] + right[1]) * 0.5];
    let outward = normalize([midpoint[0] - center[0], midpoint[1] - center[1]]);
    let outer_left = [
        left[0] + outward[0] * BRIDGE_WIDTH,
        left[1] + outward[1] * BRIDGE_WIDTH,
    ];
    let outer_right = [
        right[0] + outward[0] * BRIDGE_WIDTH,
        right[1] + outward[1] * BRIDGE_WIDTH,
    ];

    match class {
        PrototypeEdgeClass::Flat | PrototypeEdgeClass::Slope => {
            push_quad(
                data,
                [
                    [left[0], left[1], left_height],
                    [right[0], right[1], right_height],
                    [outer_right[0], outer_right[1], right_height],
                    [outer_left[0], outer_left[1], left_height],
                ],
                color_for_class(class),
            );
        }
        PrototypeEdgeClass::Cliff => {
            let low = left_height.min(right_height);
            let high = left_height.max(right_height);
            push_quad(
                data,
                [
                    [left[0], left[1], high],
                    [right[0], right[1], high],
                    [right[0], right[1], low],
                    [left[0], left[1], low],
                ],
                color_for_class(class),
            );
            push_quad(
                data,
                [
                    [left[0], left[1], low],
                    [right[0], right[1], low],
                    [outer_right[0], outer_right[1], low],
                    [outer_left[0], outer_left[1], low],
                ],
                color_for_class(class),
            );
        }
    }
}

fn push_quad(data: &mut PrototypeMeshData, vertices: [[f32; 3]; 4], color: [f32; 4]) {
    push_triangle(data, [vertices[0], vertices[1], vertices[2]], color);
    push_triangle(data, [vertices[0], vertices[2], vertices[3]], color);
}

fn append_smooth_geometry(
    data: &mut PrototypeMeshData,
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    texture_layers: Vec<[f32; 4]>,
    blend_weights: Vec<[f32; 4]>,
    indices: Vec<u32>,
) {
    let mut accumulated = vec![[0.0, 0.0, 0.0]; positions.len()];
    let mut by_position: HashMap<[i32; 3], [f32; 3]> = HashMap::new();

    for triangle in indices.chunks_exact(3) {
        let a = usize::try_from(triangle[0]).expect("local index fits in usize");
        let b = usize::try_from(triangle[1]).expect("local index fits in usize");
        let c = usize::try_from(triangle[2]).expect("local index fits in usize");
        let normal = triangle_normal([positions[a], positions[b], positions[c]]);
        for index in [a, b, c] {
            accumulated[index] = add3(accumulated[index], normal);
            let key = normal_key(positions[index]);
            let entry = by_position.entry(key).or_insert([0.0, 0.0, 0.0]);
            *entry = add3(*entry, normal);
        }
    }

    let start = u32::try_from(data.positions.len()).expect("chunk vertex count fits in u32");
    for (index, (((position, color), layers), weights)) in positions
        .into_iter()
        .zip(colors)
        .zip(texture_layers)
        .zip(blend_weights)
        .enumerate()
    {
        let key = normal_key(position);
        let normal = by_position
            .get(&key)
            .copied()
            .map_or_else(|| normalize3(accumulated[index]), normalize3);
        data.positions.push(position);
        data.normals.push(normal);
        data.colors.push(color);
        data.texture_layers.push(layers);
        data.blend_weights.push(weights);
    }
    data.indices
        .extend(indices.into_iter().map(|index| start + index));
}

fn push_triangle(data: &mut PrototypeMeshData, vertices: [[f32; 3]; 3], color: [f32; 4]) {
    let blend = solid_texture_blend(TEXTURE_LAYER_GRASSLAND);
    push_triangle_with_blends(data, vertices, [color; 3], [blend; 3]);
}

fn push_triangle_with_blends(
    data: &mut PrototypeMeshData,
    vertices: [[f32; 3]; 3],
    colors: [[f32; 4]; 3],
    blends: [TerrainBlend; 3],
) {
    let normal = triangle_normal(vertices);
    let start = u32::try_from(data.positions.len()).expect("prototype vertex count fits in u32");
    data.positions.extend_from_slice(&vertices);
    data.normals.extend_from_slice(&[normal; 3]);
    data.colors.extend_from_slice(&colors);
    data.texture_layers.extend(blends.map(|blend| blend.layers));
    data.blend_weights.extend(blends.map(|blend| blend.weights));
    data.indices
        .extend_from_slice(&[start, start + 1, start + 2]);
}

fn triangle_normal(vertices: [[f32; 3]; 3]) -> [f32; 3] {
    let a = subtract(vertices[1], vertices[0]);
    let b = subtract(vertices[2], vertices[0]);
    normalize3([
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ])
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn normal_key(position: [f32; 3]) -> [i32; 3] {
    [
        quantize_position(position[0]),
        quantize_position(position[1]),
        quantize_position(position[2]),
    ]
}

#[allow(clippy::cast_possible_truncation)]
fn quantize_position(value: f32) -> i32 {
    (value * 10_000.0).round() as i32
}

fn normalize(value: [f32; 2]) -> [f32; 2] {
    let length = (value[0].mul_add(value[0], value[1] * value[1])).sqrt();
    if length == 0.0 {
        [0.0, 1.0]
    } else {
        [value[0] / length, value[1] / length]
    }
}

fn normalize3(value: [f32; 3]) -> [f32; 3] {
    let length =
        (value[0].mul_add(value[0], value[1].mul_add(value[1], value[2] * value[2]))).sqrt();
    if length == 0.0 {
        [0.0, 0.0, 1.0]
    } else {
        [value[0] / length, value[1] / length, value[2] / length]
    }
}

fn corner_dominant_class(case: CornerPrototypeCase) -> PrototypeEdgeClass {
    let classes = [
        prototype_edge_class(case.levels[0], case.levels[1], case.forced_cliff_edges[0]),
        prototype_edge_class(case.levels[1], case.levels[2], case.forced_cliff_edges[1]),
        prototype_edge_class(case.levels[2], case.levels[0], case.forced_cliff_edges[2]),
    ];
    if classes.contains(&PrototypeEdgeClass::Cliff) {
        PrototypeEdgeClass::Cliff
    } else if classes.contains(&PrototypeEdgeClass::Slope) {
        PrototypeEdgeClass::Slope
    } else {
        PrototypeEdgeClass::Flat
    }
}

fn color_for_class(class: PrototypeEdgeClass) -> [f32; 4] {
    match class {
        PrototypeEdgeClass::Flat => [0.25, 0.55, 0.30, 1.0],
        PrototypeEdgeClass::Slope => [0.72, 0.58, 0.28, 1.0],
        PrototypeEdgeClass::Cliff => [0.46, 0.43, 0.40, 1.0],
    }
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f32(value: usize) -> f32 {
    value as f32
}

#[allow(clippy::cast_precision_loss)]
fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

#[allow(clippy::cast_precision_loss)]
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use content::Registry;
    use worldgen::{BiomeRegistry, Field, ResourceRegistry};

    use crate::sim::terrain::IMPASSABLE;

    #[test]
    fn tier_height_uses_constant_land_step() {
        assert_height_eq(tier_height(0, 5), -8.0);
        assert_height_eq(tier_height(1, 5), 0.0);
        assert_height_eq(tier_height(2, 5), 4.0);
        assert_height_eq(tier_height(5, 5), 16.0);
        assert_height_eq(tier_height(7, 5), 16.0);
    }

    #[test]
    fn prototype_suite_covers_core_corner_combinations() {
        let cases = corner_prototype_cases();
        assert!(cases.iter().any(|case| case.levels == [2, 2, 2]));
        assert!(cases.iter().any(|case| case.levels == [2, 3, 2]));
        assert!(cases.iter().any(|case| case.levels == [2, 4, 3]));
        for edge in 0..3 {
            assert!(
                cases.iter().any(|case| case.forced_cliff_edges[edge]),
                "forced cliff edge {edge} must be represented"
            );
        }
    }

    #[test]
    fn one_step_edge_is_slope_unless_forced() {
        assert_eq!(prototype_edge_class(2, 3, false), PrototypeEdgeClass::Slope);
        assert_eq!(prototype_edge_class(2, 3, true), PrototypeEdgeClass::Cliff);
    }

    #[test]
    fn prototype_mesh_indices_are_triangles_in_bounds() {
        let data = build_corner_prototype_suite_data();
        assert_vertex_attribute_lengths(&data);
        assert_eq!(data.indices.len() % 3, 0);
        let vertex_count = u32::try_from(data.positions.len()).unwrap();
        assert!(data.indices.iter().all(|index| *index < vertex_count));
    }

    #[test]
    fn terrain_chunk_rebuild_is_byte_identical() {
        let bounds = MapBounds::new(8, 6);
        let terrain = test_terrain(bounds, 5, |coord| match (coord.q + coord.r).rem_euclid(4) {
            0 => 2,
            1 => 3,
            2 => 4,
            _ => 5,
        });
        let first =
            build_terrain_chunk_data(Coord::new(0, 0), &terrain, TerrainMeshParams::default());
        let second =
            build_terrain_chunk_data(Coord::new(0, 0), &terrain, TerrainMeshParams::default());

        assert_eq!(first, second);
    }

    #[test]
    fn terrain_chunk_mesh_indices_are_triangles_in_bounds() {
        let bounds = MapBounds::new(8, 6);
        let terrain = test_terrain(bounds, 5, |coord| if coord.q == 0 { 4 } else { 2 });
        let data =
            build_terrain_chunk_data(Coord::new(0, 0), &terrain, TerrainMeshParams::default());

        assert!(!data.positions.is_empty());
        assert_vertex_attribute_lengths(&data);
        assert_eq!(data.indices.len() % 3, 0);
        let vertex_count = u32::try_from(data.positions.len()).unwrap();
        assert!(data.indices.iter().all(|index| *index < vertex_count));
    }

    #[test]
    fn terrain_chunk_contains_constant_step_heights() {
        let bounds = MapBounds::new(6, 4);
        let terrain = test_terrain(bounds, 6, |coord| {
            u8::try_from(coord.q.rem_euclid(5)).expect("small q fits in u8")
        });
        let data =
            build_terrain_chunk_data(Coord::new(0, 0), &terrain, TerrainMeshParams::default());

        for expected in [-8.0, 0.0, 4.0, 8.0, 12.0] {
            assert!(
                data.positions
                    .iter()
                    .any(|position| (position[2] - expected).abs() <= f32::EPSILON),
                "chunk should contain tier height {expected}"
            );
        }
    }

    #[test]
    fn terrain_chunk_contains_texture_blend_attributes() {
        let bounds = MapBounds::new(6, 4);
        let terrain = test_terrain(bounds, 5, |_| 2);
        let data =
            build_terrain_chunk_data(Coord::new(0, 0), &terrain, TerrainMeshParams::default());

        assert_vertex_attribute_lengths(&data);
        assert!(
            data.blend_weights
                .iter()
                .all(|weights| { (weights.iter().sum::<f32>() - 1.0).abs() <= f32::EPSILON })
        );
    }

    #[test]
    fn bridge_subdivision_carries_mixed_texture_weights() {
        let bounds = MapBounds::new(6, 4);
        let mountain = Coord::new(3, 2);
        let terrain = test_terrain(bounds, 5, |coord| if coord == mountain { 5 } else { 2 });
        let params = TerrainMeshParams {
            bridge_segments: 2,
            ..TerrainMeshParams::default()
        };
        let data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, params);

        assert!(
            data.texture_layers
                .iter()
                .zip(&data.blend_weights)
                .any(|(layers, weights)| {
                    has_weighted_layer(layers, weights, TEXTURE_LAYER_GRASSLAND)
                        && has_weighted_layer(layers, weights, TEXTURE_LAYER_MOUNTAIN)
                }),
            "subdivided grassland/mountain bridge should carry both texture layers"
        );
    }

    #[test]
    fn bridge_endpoints_keep_stable_texture_layer_order() {
        let bounds = MapBounds::new(6, 4);
        let current = Coord::new(2, 2);
        let neighbor = bounds
            .neighbor(current, Direction::East)
            .expect("current has east neighbor");
        let terrain = test_terrain(bounds, 5, |coord| if coord == neighbor { 5 } else { 2 });

        let current_blend = tile_pair_blend(&terrain, current, neighbor, 0.0);
        let neighbor_blend = tile_pair_blend(&terrain, current, neighbor, 1.0);

        assert_vec4_eq(current_blend.layers, neighbor_blend.layers);
        assert_vec4_eq(
            current_blend.layers,
            [
                f32::from(TEXTURE_LAYER_GRASSLAND),
                f32::from(TEXTURE_LAYER_MOUNTAIN),
                0.0,
                0.0,
            ],
        );
        assert_vec4_eq(current_blend.weights, [1.0, 0.0, 0.0, 0.0]);
        assert_vec4_eq(neighbor_blend.weights, [0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn flat_bridge_normals_point_up() {
        let bounds = MapBounds::new(6, 4);
        let current = Coord::new(2, 2);
        let neighbor = bounds
            .neighbor(current, Direction::East)
            .expect("current has east neighbor");
        let terrain = test_terrain(bounds, 5, |_| 2);
        let params = TerrainMeshParams::default();
        let mut data = PrototypeMeshData {
            positions: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            texture_layers: Vec::new(),
            blend_weights: Vec::new(),
            indices: Vec::new(),
        };

        push_terrain_bridge(
            &mut data,
            current,
            neighbor,
            Direction::East,
            &terrain,
            params,
        );

        assert!(!data.normals.is_empty());
        assert!(
            data.normals.iter().all(|normal| normal[2] > 0.99),
            "flat bridge front face should point upward"
        );
    }

    #[test]
    fn hilly_core_displaces_and_tapers_to_edge() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let mut terrain = test_terrain(bounds, 5, |_| 2);
        terrain.is_hilly.set(target, true);
        let params = TerrainMeshParams::default();
        let data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, params);
        let center = hex_center(target, bounds);
        let base = tier_height(2, 5);

        let max_core_height = max_height_near(&data, center, 0.0, params.core_radius * 0.35);
        assert!(
            max_core_height > base + 0.15,
            "hilly core should rise above its tier height"
        );

        let edge_height = max_height_near(
            &data,
            center,
            params.core_radius - 0.02,
            params.core_radius + 0.02,
        );
        assert_height_eq(edge_height, base);
    }

    #[test]
    fn mountain_displacement_is_larger_than_hill_displacement() {
        let bounds = MapBounds::new(6, 4);
        let hilly = Coord::new(2, 2);
        let mountain = Coord::new(3, 2);
        let mut terrain = test_terrain(bounds, 5, |coord| if coord == mountain { 5 } else { 2 });
        terrain.is_hilly.set(hilly, true);
        terrain.movement_cost.set(mountain, IMPASSABLE);
        let params = TerrainMeshParams::default();
        let data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, params);

        let hilly_peak = max_height_near(
            &data,
            hex_center(hilly, bounds),
            0.0,
            params.core_radius * 0.45,
        ) - tier_height(2, 5);
        let mountain_peak = max_height_near(
            &data,
            hex_center(mountain, bounds),
            0.0,
            params.core_radius * 0.45,
        ) - tier_height(5, 5);

        assert!(
            mountain_peak > hilly_peak * 2.0,
            "mountain displacement should visibly exceed hill displacement"
        );
    }

    #[test]
    fn mountain_edge_blends_toward_mountain_neighbor() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let neighbor = bounds
            .neighbor(target, Direction::East)
            .expect("target has east neighbor");
        let isolated = test_terrain(bounds, 5, |coord| if coord == target { 5 } else { 2 });
        let paired = test_terrain(bounds, 5, |coord| {
            if coord == target || coord == neighbor {
                5
            } else {
                2
            }
        });
        let params = TerrainMeshParams::default();
        let center = hex_center(target, bounds);
        let base_height = tier_height(5, 5);
        let edge_position =
            core_edge_midpoint(center, Direction::East, params.core_radius, base_height);

        let isolated_edge_height = displaced_core_height(
            &isolated,
            target,
            center,
            base_height,
            params.core_radius,
            params.displacement,
            edge_position,
        );
        let paired_edge_height = displaced_core_height(
            &paired,
            target,
            center,
            base_height,
            params.core_radius,
            params.displacement,
            edge_position,
        );

        assert_height_eq(isolated_edge_height, base_height);
        assert!(
            paired_edge_height > base_height + 0.1,
            "mountain edge should stay displaced across a mountain neighbor"
        );
    }

    #[test]
    fn mountain_neighbor_weight_is_capped() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let terrain = test_terrain(bounds, 5, |coord| {
            if coord == target || bounds.distance(target, coord) == 1 {
                5
            } else {
                2
            }
        });
        let displacement = TerrainDisplacementParams {
            mountain_neighbor_weight: 10.0,
            mountain_neighbor_scale_cap: 1.1,
            ..TerrainDisplacementParams::default()
        };

        assert_height_eq(
            displacement_amplitude(&terrain, target, displacement),
            displacement.mountain_amplitude * displacement.mountain_neighbor_scale_cap,
        );
    }

    #[test]
    fn custom_displacement_params_change_hill_amplitude() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let mut terrain = test_terrain(bounds, 5, |_| 2);
        terrain.is_hilly.set(target, true);
        let base_params = TerrainMeshParams::default();
        let boosted_params = TerrainMeshParams {
            displacement: TerrainDisplacementParams {
                hill_amplitude: base_params.displacement.hill_amplitude * 2.0,
                ..base_params.displacement
            },
            ..base_params
        };

        let base_data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, base_params);
        let boosted_data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, boosted_params);
        let center = hex_center(target, bounds);
        let base_height = tier_height(2, 5);
        let base_peak =
            max_height_near(&base_data, center, 0.0, base_params.core_radius * 0.45) - base_height;
        let boosted_peak = max_height_near(
            &boosted_data,
            center,
            0.0,
            boosted_params.core_radius * 0.45,
        ) - base_height;

        assert!(
            boosted_peak > base_peak * 1.5,
            "higher hill amplitude should raise hilly core displacement"
        );
    }

    #[test]
    fn zero_noise_weights_use_smooth_base_shape() {
        let params = TerrainDisplacementParams {
            noise_layers: [
                TerrainNoiseLayer {
                    weight: 0.0,
                    ..TerrainDisplacementParams::default().noise_layers[0]
                },
                TerrainNoiseLayer {
                    weight: 0.0,
                    ..TerrainDisplacementParams::default().noise_layers[1]
                },
            ],
            ..TerrainDisplacementParams::default()
        };

        assert_height_eq(displacement_noise(123.0, 456.0, params), 0.0);
    }

    #[test]
    fn noise_layer_params_are_respected() {
        let layer = TerrainNoiseLayer {
            frequency: 0.25,
            weight: 1.0,
            salt: 0xABCD_1234,
        };
        let params = TerrainDisplacementParams {
            noise_layers: [
                layer,
                TerrainNoiseLayer {
                    weight: 0.0,
                    ..TerrainDisplacementParams::default().noise_layers[1]
                },
            ],
            ..TerrainDisplacementParams::default()
        };
        let expected = value_noise(8.0 * layer.frequency, 4.0 * layer.frequency, layer.salt);

        assert_height_eq(displacement_noise(8.0, 4.0, params), expected);
    }

    #[test]
    fn displaced_core_normals_are_baked_from_geometry() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let mut terrain = test_terrain(bounds, 5, |_| 2);
        terrain.is_hilly.set(target, true);
        let data =
            build_terrain_chunk_data(Coord::new(0, 0), &terrain, TerrainMeshParams::default());

        assert!(
            data.normals
                .iter()
                .any(|normal| normal[2] > 0.0 && normal[2] < 0.999),
            "displaced geometry should produce non-flat normals"
        );
    }

    #[test]
    fn edge_ownership_is_deterministic_across_wrap_seam() {
        let bounds = MapBounds::new(8, 4);
        let west = Coord::new(0, 2);
        let east = bounds
            .neighbor(west, Direction::West)
            .expect("west neighbor wraps");

        assert_eq!(east, Coord::new(7, 2));
        assert_eq!(edge_owner(west, east), west);
        assert_eq!(edge_owner(east, west), west);
    }

    #[test]
    fn corner_ownership_is_deterministic_across_wrap_seam() {
        let bounds = MapBounds::new(8, 4);
        let coord = Coord::new(0, 2);
        let tiles = corner_tiles(coord, 3, bounds).expect("corner 3 is in bounds");

        assert_eq!(
            tiles,
            [Coord::new(0, 2), Coord::new(7, 2), Coord::new(7, 3)]
        );
        assert_eq!(corner_owner(tiles), coord);
    }

    fn assert_height_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_vec4_eq(actual: [f32; 4], expected: [f32; 4]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_height_eq(actual, expected);
        }
    }

    fn assert_vertex_attribute_lengths(data: &PrototypeMeshData) {
        assert_eq!(data.positions.len(), data.normals.len());
        assert_eq!(data.positions.len(), data.colors.len());
        assert_eq!(data.positions.len(), data.texture_layers.len());
        assert_eq!(data.positions.len(), data.blend_weights.len());
    }

    fn has_weighted_layer(layers: &[f32; 4], weights: &[f32; 4], layer: u8) -> bool {
        layers.iter().zip(weights).any(|(candidate, weight)| {
            (*candidate - f32::from(layer)).abs() <= f32::EPSILON && *weight > 0.0
        })
    }

    fn max_height_near(
        data: &PrototypeMeshData,
        center: [f32; 2],
        min_distance: f32,
        max_distance: f32,
    ) -> f32 {
        data.positions
            .iter()
            .filter(|position| {
                let dx = position[0] - center[0];
                let dy = position[1] - center[1];
                let distance = (dx.mul_add(dx, dy * dy)).sqrt();
                distance >= min_distance && distance <= max_distance
            })
            .map(|position| position[2])
            .fold(f32::NEG_INFINITY, f32::max)
    }

    fn core_edge_midpoint(
        center: [f32; 2],
        direction: Direction,
        radius: f32,
        height: f32,
    ) -> [f32; 3] {
        let [left_corner, right_corner] = edge_corners(direction);
        let left = inner_corner(center, left_corner, radius, height);
        let right = inner_corner(center, right_corner, radius, height);
        [
            (left[0] + right[0]) * 0.5,
            (left[1] + right[1]) * 0.5,
            height,
        ]
    }

    fn test_terrain(
        bounds: MapBounds,
        mountain_level: u8,
        height_at: impl Fn(Coord) -> u8,
    ) -> StaticTerrain {
        let height_level = Field::from_fn(bounds, height_at);
        let movement_cost = Field::from_fn(bounds, |coord| {
            let level = height_level.get(coord).copied().unwrap_or(0);
            if level == 0 || level == 1 || level >= mountain_level {
                IMPASSABLE
            } else {
                1
            }
        });

        StaticTerrain {
            bounds,
            mountain_level,
            height_level,
            is_hilly: Field::filled(bounds, false),
            has_river: Field::filled(bounds, false),
            is_floodplain: Field::filled(bounds, false),
            is_lake: Field::filled(bounds, false),
            biome: Field::filled(bounds, None),
            forest_density: Field::filled(bounds, 0.0),
            resource: Field::filled(bounds, None),
            movement_cost,
            start_positions: vec![],
            biome_registry: BiomeRegistry {
                inner: Registry::build([] as [&str; 0]),
                is_forest: vec![],
            },
            resource_registry: ResourceRegistry {
                inner: Registry::build([] as [&str; 0]),
                category: vec![],
            },
        }
    }
}
