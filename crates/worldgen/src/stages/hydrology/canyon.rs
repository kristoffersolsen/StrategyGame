//! Post-hoc canyon carving.

use coords::{Coord, Direction, MapBounds};

use crate::Field;
use crate::params::HydrologyParams;
use crate::world_data::RiverSize;

/// Output of the canyon carving pass.
#[derive(Debug, Clone)]
pub(crate) struct CanyonResult {
    /// Height level after canyon floors are lowered.
    pub(crate) height_level: Field<u8>,
    /// River tiles carved into canyon floors.
    pub(crate) is_canyon: Field<bool>,
    /// Tiles marked as canyon walls.
    pub(crate) is_canyon_wall: Field<bool>,
}

/// Carves canyon floors where stream power exceeds the configured threshold.
pub(crate) fn carve_canyons(
    bounds: MapBounds,
    flow_direction: &Field<Option<Direction>>,
    flow_accumulation: &Field<f32>,
    river_size: &Field<Option<RiverSize>>,
    elevation: &Field<f32>,
    height_level: &Field<u8>,
    params: &HydrologyParams,
) -> CanyonResult {
    let mut carved_height = height_level.clone();
    let mut is_canyon = Field::filled(bounds, false);
    let mut is_canyon_wall = Field::filled(bounds, false);

    for (coord, river) in river_size.iter_coords() {
        if river.is_none() {
            continue;
        }
        let Some(direction) = flow_direction[coord] else {
            continue;
        };
        let Some(downstream) = bounds.neighbor(coord, direction) else {
            continue;
        };
        let slope = (elevation[coord] - elevation[downstream]).max(0.0);
        if flow_accumulation[coord] * slope < params.canyon_flow_slope_threshold {
            continue;
        }

        is_canyon.set(coord, true);
        carved_height.set(
            coord,
            carved_height[coord]
                .saturating_sub(params.canyon_depth)
                .max(2),
        );

        for wall in canyon_wall_neighbors(bounds, coord, direction) {
            if height_level[wall] > 1 {
                is_canyon_wall.set(wall, true);
            }
        }
    }

    CanyonResult {
        height_level: carved_height,
        is_canyon,
        is_canyon_wall,
    }
}

fn canyon_wall_neighbors(bounds: MapBounds, coord: Coord, direction: Direction) -> Vec<Coord> {
    let flank_directions = match direction {
        Direction::East | Direction::West => [Direction::NorthWest, Direction::SouthEast],
        Direction::NorthEast | Direction::SouthWest => [Direction::East, Direction::West],
        Direction::NorthWest | Direction::SouthEast => [Direction::NorthEast, Direction::SouthWest],
    };

    flank_directions
        .into_iter()
        .filter_map(|flank| bounds.neighbor(coord, flank))
        .collect()
}
