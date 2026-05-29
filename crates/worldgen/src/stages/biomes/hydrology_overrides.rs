//! Hydrology-driven fertile biome overrides.

use coords::{Coord, MapBounds};

use crate::Field;
use crate::params::BiomeParams;
use crate::world_data::{BiomeHandle, HydrologyData, RiverSize};

use super::BiomeTable;

// These influence thresholds are intentional generation constants: they encode
// the relative "fertility influence" of each hydrology feature. They are NOT
// tuning knobs (changing them would require retuning the hydrology_override_strength
// param), so they stay as constants rather than moving to BiomeParams.
const FLOODPLAIN_INFLUENCE: f32 = 1.0;
const LAKE_ADJACENT_INFLUENCE: f32 = 1.0;
const MAJOR_RIVER_INFLUENCE: f32 = 1.0;
const RIVER_INFLUENCE: f32 = 0.8;
const CREEK_INFLUENCE: f32 = 0.5;

/// Applies hydrology overrides in-place to eligible biome tiles.
pub(super) fn apply_hydrology_overrides(
    bounds: MapBounds,
    hydrology: &HydrologyData,
    params: &BiomeParams,
    table: &BiomeTable,
    biome: &mut Field<Option<BiomeHandle>>,
) {
    if params.hydrology_override_strength <= 0.0 {
        return;
    }

    let original = biome.clone();
    for (coord, &value) in original.iter_coords() {
        let Some(current) = value else {
            continue;
        };
        // Dense handle-indexed lookup: O(1) instead of BTreeMap lookup.
        let Some(replacement) = table.hydrology_overrides[current.index()] else {
            continue;
        };
        let influence = hydrology_influence(bounds, hydrology, coord);
        if influence > 0.0 && params.hydrology_override_strength >= 1.0 - influence {
            biome.set(coord, Some(replacement));
        }
    }
}

fn hydrology_influence(bounds: MapBounds, hydrology: &HydrologyData, coord: Coord) -> f32 {
    let river_influence = match hydrology.river_size[coord] {
        Some(RiverSize::MajorRiver) => MAJOR_RIVER_INFLUENCE,
        Some(RiverSize::River) => RIVER_INFLUENCE,
        Some(RiverSize::Creek) => CREEK_INFLUENCE,
        None => 0.0,
    };
    let floodplain_influence = if hydrology.is_floodplain[coord] {
        FLOODPLAIN_INFLUENCE
    } else {
        0.0
    };
    let lake_influence = if is_lake_adjacent(bounds, hydrology, coord) {
        LAKE_ADJACENT_INFLUENCE
    } else {
        0.0
    };
    river_influence
        .max(floodplain_influence)
        .max(lake_influence)
}

fn is_lake_adjacent(bounds: MapBounds, hydrology: &HydrologyData, coord: Coord) -> bool {
    bounds
        .neighbors(coord)
        .into_iter()
        .flatten()
        .any(|neighbor| hydrology.is_lake[neighbor])
}
