//! Base biome lookup from continuous climate fields.

use coords::{Coord, MapBounds};

use crate::Field;
use crate::params::BiomeParams;
use crate::world_data::{BiomeHandle, ClimateData, HydrologyData};

use super::BiomeTable;

/// Assigns the initial biome to every eligible land tile.
pub(super) fn assign_base_biomes(
    bounds: MapBounds,
    climate: &ClimateData,
    hydrology: &HydrologyData,
    params: &BiomeParams,
    mountain_level: u8,
    table: &BiomeTable,
) -> Field<Option<BiomeHandle>> {
    let temperature_range = scalar_range(&climate.temperature, |coord| {
        is_biome_eligible(coord, hydrology, mountain_level)
    });
    let moisture_range = scalar_range(&climate.moisture, |coord| {
        is_biome_eligible(coord, hydrology, mountain_level)
    });

    Field::from_fn(bounds, |coord| {
        if !is_biome_eligible(coord, hydrology, mountain_level) {
            return None;
        }
        let temperature_bin = bin_value(
            climate.temperature[coord],
            temperature_range,
            params.temperature_bin_count,
        );
        let moisture_bin = bin_value(
            climate.moisture[coord],
            moisture_range,
            params.moisture_bin_count,
        );
        Some(table.lookup(temperature_bin, moisture_bin))
    })
}

fn is_biome_eligible(coord: Coord, hydrology: &HydrologyData, mountain_level: u8) -> bool {
    hydrology.height_level[coord] > 1
        && hydrology.height_level[coord] < mountain_level
        && !hydrology.is_lake[coord]
}

fn scalar_range(field: &Field<f32>, include: impl Fn(Coord) -> bool) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (coord, &value) in field.iter_coords() {
        if include(coord) {
            min = min.min(value);
            max = max.max(value);
        }
    }
    if min.is_finite() && max.is_finite() {
        (min, max)
    } else {
        (0.0, 1.0)
    }
}

fn bin_value(value: f32, (min, max): (f32, f32), bin_count: u8) -> u8 {
    let count = bin_count.max(1);
    if max <= min {
        return 0;
    }
    let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bin = (normalized * f32::from(count)).floor() as u8;
    bin.min(count - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_value_maps_to_last_bin() {
        assert_eq!(bin_value(1.0, (0.0, 1.0), 8), 7);
    }

    #[test]
    fn min_value_maps_to_first_bin() {
        assert_eq!(bin_value(0.0, (0.0, 1.0), 8), 0);
    }
}
