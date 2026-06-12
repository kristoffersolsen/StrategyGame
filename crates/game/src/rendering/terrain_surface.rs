//! Continuous terrain surface mesh generation.
//!
//! Implements the R1–R3 terrain geometry pipeline:
//! - R1/R2: corner-primary lattice with shared corners and edge bridges.
//! - R3.1: directional ridge field for hill tiles — flowing ridges at constant tier height.
//! - R3.2: corner-and-edge crest skeleton for mountain tiles — raised shared corners form ridgelines.
//! - R3.3: smoothstep bridge profiles for slopes; hard top lip for cliff edges.

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
const DEFAULT_SLOPE_BRIDGE_SEGMENTS: u8 = 3;
const DEFAULT_CLIFF_BRIDGE_SEGMENTS: u8 = 1;

// Hill directional ridge field defaults (R3.1)
const DEFAULT_HILL_AMPLITUDE: f32 = 1.5;
const DEFAULT_HILL_WAVELENGTH: f32 = 3.5;
const DEFAULT_HILL_FIELD_RATE: f32 = 0.022;
const DEFAULT_HILL_WARP_STRENGTH: f32 = 3.5;
const DEFAULT_HILL_WARP_SALT_X: u32 = 0x73A1_C4D2;
const DEFAULT_HILL_WARP_SALT_Y: u32 = 0x9F2B_5E18;
const DEFAULT_HILL_DIR_SALT: u32 = 0x4A7D_B3C1;
const DEFAULT_HILL_DETAIL_AMPLITUDE: f32 = 0.5;
const DEFAULT_HILL_DETAIL_FREQUENCY: f32 = 0.22;
const DEFAULT_HILL_DETAIL_SALT: u32 = 0xC823_74AB;

// Mountain crest skeleton defaults (R3.2)
const DEFAULT_MOUNTAIN_CREST_HEIGHT: f32 = 6.0;
const DEFAULT_MOUNTAIN_EDGE_ARCH: f32 = 0.65;
const DEFAULT_MOUNTAIN_DETAIL_AMPLITUDE: f32 = 1.0;
const DEFAULT_MOUNTAIN_DETAIL_FREQUENCY: f32 = 0.10;
const DEFAULT_MOUNTAIN_DETAIL_SALT: u32 = 0xF148_2D6A;

// Cliff lip geometry defaults (R3.3)
const DEFAULT_CLIFF_LIP_HEIGHT: f32 = 1.5;
const DEFAULT_CLIFF_LIP_WIDTH: f32 = 1.2;
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

/// Parameters for the hill directional ridge field (R3.1).
///
/// The ridge field is a pure function of world XY position: domain-warped
/// noise drives a slowly-rotating direction field; a ridged cosine profile
/// advances perpendicular to that field, producing flowing ridges at constant
/// tier height. Amplitude fades to zero at hill→non-hill edges.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HillRidgeParams {
    /// Peak-to-valley height of each ridge.
    pub(crate) amplitude: f32,
    /// Distance between ridge crests in world units.
    pub(crate) wavelength: f32,
    /// Spatial rate at which the direction field rotates.
    pub(crate) field_rate: f32,
    /// Magnitude of the domain-warp offset applied before sampling the field.
    pub(crate) warp_strength: f32,
    /// Salts for the two warp-offset noise lookups.
    pub(crate) warp_salt: [u32; 2],
    /// Salt for the direction-angle noise lookup.
    pub(crate) dir_salt: u32,
    /// Amplitude of the top-detail noise layer.
    pub(crate) detail_amplitude: f32,
    /// Spatial frequency of the top-detail noise layer.
    pub(crate) detail_frequency: f32,
    /// Salt for the top-detail noise layer.
    pub(crate) detail_salt: u32,
}

impl Default for HillRidgeParams {
    fn default() -> Self {
        Self {
            amplitude: DEFAULT_HILL_AMPLITUDE,
            wavelength: DEFAULT_HILL_WAVELENGTH,
            field_rate: DEFAULT_HILL_FIELD_RATE,
            warp_strength: DEFAULT_HILL_WARP_STRENGTH,
            warp_salt: [DEFAULT_HILL_WARP_SALT_X, DEFAULT_HILL_WARP_SALT_Y],
            dir_salt: DEFAULT_HILL_DIR_SALT,
            detail_amplitude: DEFAULT_HILL_DETAIL_AMPLITUDE,
            detail_frequency: DEFAULT_HILL_DETAIL_FREQUENCY,
            detail_salt: DEFAULT_HILL_DETAIL_SALT,
        }
    }
}

/// Parameters for the mountain corner-and-edge crest skeleton (R3.2).
///
/// Corners and shared edges between mountain tiles are elevated toward
/// `crest_height`, forming a connected graph of ridgelines. Mountain→lower
/// edges fall away as flanks. 3–4 deterministic interior variants add
/// secondary character (asymmetry, concavity, subsidiary ridges).
#[derive(Debug, Clone, Copy)]
pub(crate) struct MountainCrestParams {
    /// Maximum height above tier at positions fully surrounded by mountains.
    pub(crate) crest_height: f32,
    /// Arch fraction added at mountain–mountain bridge midpoints (0..1).
    pub(crate) edge_arch: f32,
    /// Amplitude of the top-detail noise layer.
    pub(crate) detail_amplitude: f32,
    /// Spatial frequency of the top-detail noise layer.
    pub(crate) detail_frequency: f32,
    /// Salt for the top-detail noise layer.
    pub(crate) detail_salt: u32,
}

impl Default for MountainCrestParams {
    fn default() -> Self {
        Self {
            crest_height: DEFAULT_MOUNTAIN_CREST_HEIGHT,
            edge_arch: DEFAULT_MOUNTAIN_EDGE_ARCH,
            detail_amplitude: DEFAULT_MOUNTAIN_DETAIL_AMPLITUDE,
            detail_frequency: DEFAULT_MOUNTAIN_DETAIL_FREQUENCY,
            detail_salt: DEFAULT_MOUNTAIN_DETAIL_SALT,
        }
    }
}

/// Terrain mesh construction parameters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainMeshParams {
    /// Radius of each tile's inner hexagonal core.
    pub(crate) core_radius: f32,
    /// Subdivisions applied to each core sector triangle.
    pub(crate) core_subdivisions: u8,
    /// Bridge segments for slope edges (more resolves the rounded profile).
    pub(crate) slope_bridge_segments: u8,
    /// Bridge segments for cliff edges (fewer; vertical faces need little subdivision).
    pub(crate) cliff_bridge_segments: u8,
    /// Height of the hard lip added to the top of cliff faces.
    pub(crate) cliff_lip_height: f32,
    /// Horizontal extension of the cliff lip toward the lower tile.
    pub(crate) cliff_lip_width: f32,
    /// Hill directional ridge field parameters.
    pub(crate) hill_ridge: HillRidgeParams,
    /// Mountain crest skeleton parameters.
    pub(crate) mountain_crest: MountainCrestParams,
}

impl Default for TerrainMeshParams {
    fn default() -> Self {
        Self {
            core_radius: HEX_SIZE * CORE_RADIUS_RATIO,
            core_subdivisions: DEFAULT_CORE_SUBDIVISIONS,
            slope_bridge_segments: DEFAULT_SLOPE_BRIDGE_SEGMENTS,
            cliff_bridge_segments: DEFAULT_CLIFF_BRIDGE_SEGMENTS,
            cliff_lip_height: DEFAULT_CLIFF_LIP_HEIGHT,
            cliff_lip_width: DEFAULT_CLIFF_LIP_WIDTH,
            hill_ridge: HillRidgeParams::default(),
            mountain_crest: MountainCrestParams::default(),
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
                hill_ridge: params.hill_ridge,
                mountain_crest: params.mountain_crest,
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

    // Cliff edges get fewer bridge segments; slopes get more to resolve the rounded profile.
    let cross_segments = match class {
        EdgeClass::Flat | EdgeClass::Slope => params.slope_bridge_segments,
        EdgeClass::Cliff => params.cliff_bridge_segments,
    };
    let is_slope = class != EdgeClass::Cliff;

    // Mountain–mountain bridges get an arch at the midpoint (R3.2 crest continuation).
    let mountain_arch = if is_mountain_tile(terrain, coord) && is_mountain_tile(terrain, neighbor) {
        params.mountain_crest.edge_arch * params.mountain_crest.crest_height
    } else {
        0.0
    };

    push_bridge_grid(
        data,
        BridgeGrid {
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
            hill_ridge: params.hill_ridge,
            mountain_crest: params.mountain_crest,
            mountain_arch,
            side_segments: params.core_subdivisions,
            cross_segments,
            is_slope,
            color,
        },
    );

    // Add a hard top lip to cliff edges (R3.3).
    if class == EdgeClass::Cliff && params.cliff_lip_height > 0.0 {
        push_cliff_lip(
            data,
            center,
            neighbor_center,
            current_left,
            current_right,
            neighbor_left,
            neighbor_right,
            terrain,
            coord,
            neighbor,
            params.core_radius,
            params.hill_ridge,
            params.mountain_crest,
            params.cliff_lip_height,
            params.cliff_lip_width,
            color,
        );
    }
}

#[derive(Clone, Copy)]
struct BridgeGrid<'a> {
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
    hill_ridge: HillRidgeParams,
    mountain_crest: MountainCrestParams,
    /// Additional arch height at the bridge midpoint for mountain–mountain edges.
    mountain_arch: f32,
    side_segments: u8,
    cross_segments: u8,
    /// When true, apply smoothstep profile to round the bridge shoulders (R3.3).
    is_slope: bool,
    color: [f32; 4],
}

fn push_bridge_grid(data: &mut PrototypeMeshData, grid: BridgeGrid) {
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
        let current_xy = lerp2(grid.current_left, grid.current_right, side_t);
        let neighbor_xy = lerp2(grid.neighbor_left, grid.neighbor_right, side_t);
        let h_current = core_height_at(
            grid.terrain, grid.current, grid.current_center,
            grid.radius, grid.hill_ridge, grid.mountain_crest, current_xy,
        );
        let h_neighbor = core_height_at(
            grid.terrain, grid.neighbor, grid.neighbor_center,
            grid.radius, grid.hill_ridge, grid.mountain_crest, neighbor_xy,
        );

        for cross in 0..=cross_segments {
            let cross_t = u32_to_f32(cross) / u32_to_f32(cross_segments);
            // R3.3: smoothstep profile for slopes — rounds the shoulder crease at core joins.
            let profile_t = if grid.is_slope {
                smoothstep(0.0, 1.0, cross_t)
            } else {
                cross_t
            };
            // Mountain–mountain bridges arch upward at the midpoint (R3.2).
            let arch = grid.mountain_arch * 4.0 * cross_t * (1.0 - cross_t);
            let xy = lerp2(current_xy, neighbor_xy, cross_t);
            let z = lerp(h_current, h_neighbor, profile_t) + arch;
            positions.push([xy[0], xy[1], z]);
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

/// Adds a hard horizontal lip at the top of a cliff edge (R3.3).
///
/// The lip sits on the high side of the cliff — a small horizontal overhang
/// that makes the impassable drop legible through silhouette and lighting.
#[allow(clippy::too_many_arguments)]
fn push_cliff_lip(
    data: &mut PrototypeMeshData,
    current_center: [f32; 2],
    neighbor_center: [f32; 2],
    current_left: [f32; 2],
    current_right: [f32; 2],
    neighbor_left: [f32; 2],
    neighbor_right: [f32; 2],
    terrain: &StaticTerrain,
    current: Coord,
    neighbor: Coord,
    radius: f32,
    hill_ridge: HillRidgeParams,
    mountain_crest: MountainCrestParams,
    lip_height: f32,
    lip_width: f32,
    color: [f32; 4],
) {
    let h_current_l = core_height_at(terrain, current, current_center, radius, hill_ridge, mountain_crest, current_left);
    let h_current_r = core_height_at(terrain, current, current_center, radius, hill_ridge, mountain_crest, current_right);
    let h_neighbor_l = core_height_at(terrain, neighbor, neighbor_center, radius, hill_ridge, mountain_crest, neighbor_left);
    let h_neighbor_r = core_height_at(terrain, neighbor, neighbor_center, radius, hill_ridge, mountain_crest, neighbor_right);

    let h_current_avg = (h_current_l + h_current_r) * 0.5;
    let h_neighbor_avg = (h_neighbor_l + h_neighbor_r) * 0.5;

    // Lip belongs to the higher side.
    let (high_left, high_right, h_high_l, h_high_r, low_left, low_right) =
        if h_current_avg >= h_neighbor_avg {
            (current_left, current_right, h_current_l, h_current_r, neighbor_left, neighbor_right)
        } else {
            (neighbor_left, neighbor_right, h_neighbor_l, h_neighbor_r, current_left, current_right)
        };

    let high_mid = lerp2(high_left, high_right, 0.5);
    let low_mid = lerp2(low_left, low_right, 0.5);
    let outward = normalize([low_mid[0] - high_mid[0], low_mid[1] - high_mid[1]]);

    // Lip extends slightly toward the lower tile.
    let lip_left = [high_left[0] + outward[0] * lip_width, high_left[1] + outward[1] * lip_width];
    let lip_right = [high_right[0] + outward[0] * lip_width, high_right[1] + outward[1] * lip_width];
    let lip_z_l = h_high_l + lip_height;
    let lip_z_r = h_high_r + lip_height;

    // Top of lip (horizontal face, upward-facing).
    push_triangle_with_blends(
        data,
        [[high_left[0], high_left[1], lip_z_l], [high_right[0], high_right[1], lip_z_r], [lip_right[0], lip_right[1], lip_z_r]],
        [color; 3],
        [solid_texture_blend(TEXTURE_LAYER_MOUNTAIN); 3],
    );
    push_triangle_with_blends(
        data,
        [[high_left[0], high_left[1], lip_z_l], [lip_right[0], lip_right[1], lip_z_r], [lip_left[0], lip_left[1], lip_z_l]],
        [color; 3],
        [solid_texture_blend(TEXTURE_LAYER_MOUNTAIN); 3],
    );

    // Outer face of lip (faces outward, hard face normal).
    push_triangle_with_blends(
        data,
        [[lip_left[0], lip_left[1], h_high_l], [lip_right[0], lip_right[1], h_high_r], [lip_right[0], lip_right[1], lip_z_r]],
        [color; 3],
        [solid_texture_blend(TEXTURE_LAYER_MOUNTAIN); 3],
    );
    push_triangle_with_blends(
        data,
        [[lip_left[0], lip_left[1], h_high_l], [lip_right[0], lip_right[1], lip_z_r], [lip_left[0], lip_left[1], lip_z_l]],
        [color; 3],
        [solid_texture_blend(TEXTURE_LAYER_MOUNTAIN); 3],
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
        core_position_toward(
            terrain,
            tiles[0],
            centers[0],
            corner_point,
            params.core_radius,
            params.hill_ridge,
            params.mountain_crest,
        ),
        core_position_toward(
            terrain,
            tiles[1],
            centers[1],
            corner_point,
            params.core_radius,
            params.hill_ridge,
            params.mountain_crest,
        ),
        core_position_toward(
            terrain,
            tiles[2],
            centers[2],
            corner_point,
            params.core_radius,
            params.hill_ridge,
            params.mountain_crest,
        ),
    ];
    let color = corner_color(terrain, tiles);
    let blend = weighted_tile_blend(
        terrain,
        &[(tiles[0], 1.0), (tiles[1], 1.0), (tiles[2], 1.0)],
    );
    push_triangle_with_blends(data, vertices, [color; 3], [blend; 3]);
}

fn core_position_toward(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    target: [f32; 2],
    radius: f32,
    hill_ridge: HillRidgeParams,
    mountain_crest: MountainCrestParams,
) -> [f32; 3] {
    let xy = point_toward_xy(center, target, radius);
    [xy[0], xy[1], core_height_at(terrain, coord, center, radius, hill_ridge, mountain_crest, xy)]
}

/// Dispatches to the correct height function for the tile's terrain type.
fn core_height_at(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    radius: f32,
    hill_ridge: HillRidgeParams,
    mountain_crest: MountainCrestParams,
    xy: [f32; 2],
) -> f32 {
    let base = terrain_height(terrain, coord);
    let level = terrain.height_level.get(coord).copied().unwrap_or(0);
    if level >= terrain.mountain_level {
        mountain_core_height(terrain, coord, center, base, radius, mountain_crest, xy)
    } else if terrain.is_hilly.get(coord).copied().unwrap_or(false) {
        hill_core_height(terrain, coord, center, base, radius, hill_ridge, xy)
    } else {
        base
    }
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
    hill_ridge: HillRidgeParams,
    mountain_crest: MountainCrestParams,
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
                core_height_at(
                    sector.terrain,
                    sector.coord,
                    sector.center,
                    sector.radius,
                    sector.hill_ridge,
                    sector.mountain_crest,
                    [position[0], position[1]],
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

// ---- Hill ridge field (R3.1) ----

/// Computes the height of a hill-tile core vertex at world position `xy`.
///
/// The ridge field is a pure function of XY, so independent chunks agree at
/// their seams. Amplitude fades to zero at hill→non-hill edges, keeping the
/// mean height at tier level and preventing visual walls.
fn hill_core_height(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    base: f32,
    radius: f32,
    params: HillRidgeParams,
    xy: [f32; 2],
) -> f32 {
    let offset = [xy[0] - center[0], xy[1] - center[1]];
    let fade = hill_edge_fade(terrain, coord, offset, radius);
    if fade <= 0.0 {
        return base;
    }
    let ridge = hill_ridge_displacement(xy, params);
    let detail = params.detail_amplitude
        * value_noise(xy[0] * params.detail_frequency, xy[1] * params.detail_frequency, params.detail_salt);
    base + fade * (ridge + detail)
}

/// Domain-warped directional ridge profile — a pure function of world XY.
///
/// Returns a displacement in world-height units, symmetric around zero
/// (ridges go above and below base; mean = 0 so tier height is preserved).
fn hill_ridge_displacement(xy: [f32; 2], params: HillRidgeParams) -> f32 {
    // Domain warp: slow noise offset so ridges meander rather than running straight.
    let warp_freq = params.field_rate * 2.5;
    let wx = value_noise(xy[0] * warp_freq, xy[1] * warp_freq, params.warp_salt[0]);
    let wy = value_noise(xy[0] * warp_freq, xy[1] * warp_freq + 13.71, params.warp_salt[1]);
    let warped = [
        xy[0] + params.warp_strength * wx,
        xy[1] + params.warp_strength * wy,
    ];

    // Direction field: slowly-rotating angle driven by low-frequency noise.
    let angle = value_noise(
        warped[0] * params.field_rate,
        warped[1] * params.field_rate,
        params.dir_salt,
    ) * std::f32::consts::TAU;

    // Ridged cosine: range [-1, 1], mean 0, peaks at integer phase values.
    let phase = dot2(warped, [angle.cos(), angle.sin()]) / params.wavelength;
    (std::f32::consts::TAU * phase).cos() * params.amplitude
}

/// Returns the amplitude-fade factor [0..1] for a position in a hill tile.
///
/// Fades to 0 at hill→non-hill edges; stays 1 across hill→hill edges,
/// so ridges flow continuously between adjacent hill tiles.
fn hill_edge_fade(
    terrain: &StaticTerrain,
    coord: Coord,
    offset: [f32; 2],
    radius: f32,
) -> f32 {
    // Base taper: falls to 0 at the core boundary regardless of neighbors.
    let edge_progress = hex_edge_progress(offset, radius);
    let base_taper = 1.0 - smoothstep(0.1, 1.0, edge_progress);

    // Edge-support boost: hold amplitude up at hill→hill (and hill→mountain) edges.
    let apothem = hex_apothem(radius);
    let mut total_weight = 0.0_f32;
    let mut total_support = 0.0_f32;

    for direction in GEOMETRY_DIRECTIONS {
        let side_weight = (dot2(offset, direction_unit(direction)) / apothem)
            .max(0.0)
            .powi(4);
        if side_weight <= f32::EPSILON {
            continue;
        }
        total_weight += side_weight;
        let neighbor_is_ridged = terrain.bounds.neighbor(coord, direction).is_some_and(|n| {
            let nl = terrain.height_level.get(n).copied().unwrap_or(0);
            nl >= terrain.mountain_level || terrain.is_hilly.get(n).copied().unwrap_or(false)
        });
        if neighbor_is_ridged {
            total_support += side_weight;
        }
    }

    let support = if total_weight > f32::EPSILON {
        total_support / total_weight
    } else {
        0.0
    };
    base_taper + (1.0 - base_taper) * support
}

// ---- Mountain crest skeleton (R3.2) ----

/// Computes the height of a mountain-tile core vertex at world position `xy`.
///
/// Corners and edges shared between mountain tiles are elevated toward
/// `crest_height`, forming a connected ridgeline across mountain clusters.
/// At mountain→lower edges, displacement falls to zero (flanks). Interior
/// vertices receive a dome-shaped base raised by a deterministic variant
/// for secondary character.
fn mountain_core_height(
    terrain: &StaticTerrain,
    coord: Coord,
    center: [f32; 2],
    base: f32,
    radius: f32,
    params: MountainCrestParams,
    xy: [f32; 2],
) -> f32 {
    let dx = xy[0] - center[0];
    let dy = xy[1] - center[1];

    // `edge_progress` is 0 at the tile centre, 1 at any point on the hex core boundary.
    // `dome` uses this so it reaches exactly 0 at every core-edge point (including
    // the apothem midpoints, not just the circumradius corners).
    let edge_progress = hex_edge_progress([dx, dy], radius);
    let dome = 1.0 - edge_progress;

    // Edge support: each hex face weighted by proximity and mountain-ness of neighbor.
    let apothem = hex_apothem(radius);
    let mut total_weight = 0.0_f32;
    let mut total_support = 0.0_f32;

    for direction in GEOMETRY_DIRECTIONS {
        let side_weight = (dot2([dx, dy], direction_unit(direction)) / apothem)
            .max(0.0)
            .powi(4);
        if side_weight <= f32::EPSILON {
            continue;
        }
        total_weight += side_weight;
        if terrain.bounds.neighbor(coord, direction).is_some_and(|n| is_mountain_tile(terrain, n)) {
            total_support += side_weight;
        }
    }

    let edge_support = if total_weight > f32::EPSILON {
        total_support / total_weight
    } else {
        0.0
    };

    // At interior points the dome dominates; at boundaries edge_support * edge_progress
    // takes over — rising to crest_height at mountain-mountain edges and falling to zero
    // at mountain-lower edges.
    let crest_fraction = dome.max(edge_support * edge_progress);

    let variant = mountain_interior_variant(coord, dx, dy, radius, params.crest_height);
    let detail = params.detail_amplitude
        * value_noise(xy[0] * params.detail_frequency, xy[1] * params.detail_frequency, params.detail_salt);

    // Scale all displacement by crest_fraction so it falls to zero at non-mountain edges.
    let offset = (params.crest_height + variant + detail) * crest_fraction;
    base + offset.max(0.0)
}

/// Deterministic secondary-character modifier for mountain tile interiors.
///
/// Selects one of four variant shapes based on a hash of the tile coord and
/// applies a hex-rotation so ranges don't read as repetitive stripes.
fn mountain_interior_variant(coord: Coord, dx: f32, dy: f32, radius: f32, crest_height: f32) -> f32 {
    let hash = tile_hash(coord);
    let [rdx, rdy] = rotate_hex_steps([dx, dy], hash % 6);
    let rx = rdx / radius;
    let ry = rdy / radius;
    let scale = crest_height * 0.10;

    match (hash >> 3) % 4 {
        0 => 0.0,
        1 => scale * ry.clamp(-1.0, 1.0),                                        // asymmetric lean
        2 => -scale * (rx * rx + ry * ry).min(1.0),                              // bowl concavity
        3 => scale * (1.0 - (rx + 0.35).abs() * 2.0).max(0.0),                  // subsidiary ridge
        _ => unreachable!("hash % 4 is 0..3"),
    }
}

fn tile_hash(coord: Coord) -> u32 {
    let mut v = u32::from_ne_bytes(coord.q.to_ne_bytes()).wrapping_mul(0x9E37_79B1);
    v ^= u32::from_ne_bytes(coord.r.to_ne_bytes()).wrapping_mul(0x6C62_272E);
    v ^= v >> 16;
    v = v.wrapping_mul(0x7FEB_352D);
    v ^= v >> 15;
    v
}

fn rotate_hex_steps(xy: [f32; 2], steps: u32) -> [f32; 2] {
    let angle = u32_to_f32(steps % 6) * std::f32::consts::PI / 3.0;
    let (sin, cos) = angle.sin_cos();
    [cos * xy[0] - sin * xy[1], sin * xy[0] + cos * xy[1]]
}

fn is_mountain_tile(terrain: &StaticTerrain, coord: Coord) -> bool {
    terrain.height_level.get(coord).copied().unwrap_or(0) >= terrain.mountain_level
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
            slope_bridge_segments: 2,
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

    // R3.1 — Hill ridge field tests

    #[test]
    fn hilly_core_shows_ridge_variation_and_fades_at_edge() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let mut terrain = test_terrain(bounds, 5, |_| 2);
        terrain.is_hilly.set(target, true);
        let params = TerrainMeshParams::default();
        let data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, params);
        let center = hex_center(target, bounds);
        let base = tier_height(2, 5);

        // Inner region covers > 1 ridge wavelength so it must contain both peaks and valleys.
        let inner_radius = params.core_radius * 0.5;
        let max_inner = max_height_near(&data, center, 0.0, inner_radius);
        let min_inner = min_height_near(&data, center, 0.0, inner_radius);
        assert!(
            max_inner - min_inner > params.hill_ridge.amplitude * 0.5,
            "hill core should show significant ridge variation (max={max_inner:.3}, min={min_inner:.3})"
        );

        // At the core boundary toward non-hilly neighbors, amplitude fades to zero.
        let edge_max = max_height_near(&data, center, params.core_radius - 0.02, params.core_radius + 0.02);
        assert_height_eq(edge_max, base);
    }

    #[test]
    fn hill_ridge_amplitude_scales_proportionally() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let mut terrain = test_terrain(bounds, 5, |_| 2);
        terrain.is_hilly.set(target, true);
        let base_params = TerrainMeshParams::default();
        let boosted_params = TerrainMeshParams {
            hill_ridge: HillRidgeParams {
                amplitude: base_params.hill_ridge.amplitude * 2.0,
                detail_amplitude: 0.0,
                ..base_params.hill_ridge
            },
            ..base_params
        };
        let no_detail_params = TerrainMeshParams {
            hill_ridge: HillRidgeParams { detail_amplitude: 0.0, ..base_params.hill_ridge },
            ..base_params
        };

        let center = hex_center(target, bounds);
        let inner = base_params.core_radius * 0.5;
        let base_range = max_height_near(
            &build_terrain_chunk_data(Coord::new(0, 0), &terrain, no_detail_params),
            center, 0.0, inner,
        ) - min_height_near(
            &build_terrain_chunk_data(Coord::new(0, 0), &terrain, no_detail_params),
            center, 0.0, inner,
        );
        let boosted_range = max_height_near(
            &build_terrain_chunk_data(Coord::new(0, 0), &terrain, boosted_params),
            center, 0.0, inner,
        ) - min_height_near(
            &build_terrain_chunk_data(Coord::new(0, 0), &terrain, boosted_params),
            center, 0.0, inner,
        );

        assert!(
            boosted_range > base_range * 1.5,
            "doubling hill amplitude should visibly widen the height range"
        );
    }

    // R3.2 — Mountain crest skeleton tests

    #[test]
    fn mountain_crest_exceeds_hill_amplitude() {
        let bounds = MapBounds::new(6, 4);
        let hilly = Coord::new(2, 2);
        let mountain = Coord::new(3, 2);
        let mut terrain = test_terrain(bounds, 5, |coord| if coord == mountain { 5 } else { 2 });
        terrain.is_hilly.set(hilly, true);
        terrain.movement_cost.set(mountain, IMPASSABLE);
        let params = TerrainMeshParams::default();
        let data = build_terrain_chunk_data(Coord::new(0, 0), &terrain, params);

        let hill_range = {
            let c = hex_center(hilly, bounds);
            let r = params.core_radius * 0.5;
            max_height_near(&data, c, 0.0, r) - min_height_near(&data, c, 0.0, r)
        };
        let mountain_peak = max_height_near(
            &data, hex_center(mountain, bounds), 0.0, params.core_radius * 0.5,
        ) - tier_height(5, 5);

        assert!(
            mountain_peak > hill_range * 2.0,
            "mountain crest height should greatly exceed hill ridge range \
             (mountain_peak={mountain_peak:.2}, hill_range={hill_range:.2})"
        );
    }

    #[test]
    fn mountain_edge_is_flat_for_isolated_tile_and_elevated_for_paired() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(2, 2);
        let neighbor = bounds
            .neighbor(target, Direction::East)
            .expect("target has east neighbor");
        let isolated = test_terrain(bounds, 5, |coord| if coord == target { 5 } else { 2 });
        let paired = test_terrain(bounds, 5, |coord| {
            if coord == target || coord == neighbor { 5 } else { 2 }
        });
        let params = TerrainMeshParams::default();
        let center = hex_center(target, bounds);
        let base_height = tier_height(5, 5);
        let [ex, ey, _] = core_edge_midpoint(center, Direction::East, params.core_radius, base_height);

        let isolated_h = core_height_at(
            &isolated, target, center, params.core_radius,
            params.hill_ridge, params.mountain_crest, [ex, ey],
        );
        let paired_h = core_height_at(
            &paired, target, center, params.core_radius,
            params.hill_ridge, params.mountain_crest, [ex, ey],
        );

        assert_height_eq(isolated_h, base_height);
        assert!(
            paired_h > base_height + 0.1,
            "mountain edge toward a mountain neighbor should be elevated above tier"
        );
    }

    #[test]
    fn fully_surrounded_mountain_has_higher_center_than_isolated() {
        let bounds = MapBounds::new(6, 4);
        let target = Coord::new(3, 2);
        let isolated_terrain = test_terrain(bounds, 5, |coord| if coord == target { 5 } else { 2 });
        let surrounded_terrain = test_terrain(bounds, 5, |coord| {
            if coord == target || bounds.distance(target, coord) == 1 { 5 } else { 2 }
        });
        let params = TerrainMeshParams::default();
        let center = hex_center(target, bounds);

        // Zero-amplitude detail so the comparison is purely from crest skeleton.
        let crest_only = MountainCrestParams { detail_amplitude: 0.0, ..params.mountain_crest };
        let params_nd = TerrainMeshParams { mountain_crest: crest_only, ..params };

        let isolated_peak = core_height_at(
            &isolated_terrain, target, center, params_nd.core_radius,
            params_nd.hill_ridge, params_nd.mountain_crest, center,
        );
        let surrounded_peak = core_height_at(
            &surrounded_terrain, target, center, params_nd.core_radius,
            params_nd.hill_ridge, params_nd.mountain_crest, center,
        );

        // Both should be elevated (dome at center), but heights are identical since dome
        // is independent of neighbors; the difference shows at edges, not the center.
        // This test verifies that surrounded still reaches crest_height at center.
        assert!(
            surrounded_peak > tier_height(5, 5) + params_nd.mountain_crest.crest_height * 0.8,
            "fully-surrounded mountain center should reach near crest_height"
        );
        assert!(
            surrounded_peak >= isolated_peak,
            "surrounded mountain should be at least as tall as isolated"
        );
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
        height_near_fold(data, center, min_distance, max_distance, f32::NEG_INFINITY, f32::max)
    }

    fn min_height_near(
        data: &PrototypeMeshData,
        center: [f32; 2],
        min_distance: f32,
        max_distance: f32,
    ) -> f32 {
        height_near_fold(data, center, min_distance, max_distance, f32::INFINITY, f32::min)
    }

    fn height_near_fold(
        data: &PrototypeMeshData,
        center: [f32; 2],
        min_distance: f32,
        max_distance: f32,
        init: f32,
        fold_fn: impl Fn(f32, f32) -> f32,
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
            .fold(init, fold_fn)
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
