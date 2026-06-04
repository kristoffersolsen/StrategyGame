//! Shared pointy-top hex coordinate helpers for rendering.

use coords::{Coord, MapBounds};

use crate::{rendering::terrain_surface::tier_height, sim::terrain::StaticTerrain};

/// Circumradius of one pointy-top hex in world units.
pub(crate) const HEX_SIZE: f32 = 16.0;

/// Returns the approximate world-space size of the generated map.
pub(crate) fn map_world_size(bounds: MapBounds) -> (f32, f32) {
    let width = f32::sqrt(3.0) * HEX_SIZE * (u32_to_f32(bounds.width()) + 1.0);
    let height = HEX_SIZE * (1.5 * (u32_to_f32(bounds.height()) - 1.0) + 2.0);
    (width, height)
}

/// Returns the center point for a tile on the terrain tier surface.
pub(crate) fn tile_center_position(terrain: &StaticTerrain, coord: Coord) -> [f32; 3] {
    let center = hex_center(coord, terrain.bounds);
    let height_level = terrain.height_level.get(coord).copied().unwrap_or(0);
    [
        center[0],
        center[1],
        tier_height(height_level, terrain.mountain_level),
    ]
}

pub(crate) fn hex_center(coord: Coord, bounds: MapBounds) -> [f32; 2] {
    let q = i32_to_f32(coord.q);
    let r = i32_to_f32(coord.r);
    let wrapped_x = (q + 0.5 * r).rem_euclid(u32_to_f32(bounds.width()));
    [
        HEX_SIZE * f32::sqrt(3.0) * wrapped_x + HEX_SIZE * f32::sqrt(3.0) * 0.5,
        -HEX_SIZE * (1.5 * r + 1.0),
    ]
}

#[allow(clippy::cast_precision_loss)]
fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

#[allow(clippy::cast_precision_loss)]
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}
