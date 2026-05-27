//! Floodplain feature marking.

use coords::MapBounds;

use crate::Field;
use crate::params::HydrologyParams;
use crate::world_data::RiverSize;

/// Marks land tiles adjacent to rivers and close in elevation.
pub(crate) fn mark_floodplains(
    bounds: MapBounds,
    river_size: &Field<Option<RiverSize>>,
    elevation: &Field<f32>,
    height_level: &Field<u8>,
    params: &HydrologyParams,
) -> Field<bool> {
    Field::from_fn(bounds, |coord| {
        if height_level[coord] <= 1 || river_size[coord].is_some() {
            return false;
        }

        bounds.neighbors(coord).iter().flatten().any(|&neighbor| {
            river_size[neighbor].is_some()
                && (elevation[coord] - elevation[neighbor]).abs()
                    <= params.floodplain_elevation_tolerance
        })
    })
}
