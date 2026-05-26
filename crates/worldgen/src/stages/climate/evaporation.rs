//! Evaporation source field construction (step 3).
//!
//! Every water tile (ocean, coast — `height_level` 0 or 1) acts as a
//! moisture source. The evaporation rate is scaled by temperature:
//! cold water evaporates less; tiles below `evaporation_temp_floor`
//! contribute nothing.
//!
//! The field is computed once and consumed by every advection iteration.

use coords::{Coord, MapBounds};

use crate::Field;
use crate::params::ClimateParams;

// ------------- Water-tile threshold -------------

/// Maximum height level classified as a water source.
///
/// Levels 0 (ocean) and 1 (coast) are water; all higher levels are land.
/// Per design §1.10: "All water tiles are moisture sources, not just
/// ocean. Inland seas evaporate."
const MAX_WATER_LEVEL: u8 = 1;

// ------------- Public entry point -------------

/// Builds the per-tile evaporation source field.
///
/// Non-water tiles have evaporation 0. Water tiles have:
///
/// ```text
/// evap = evaporation_rate
///      × clamp((temperature − evaporation_temp_floor)
///              × evaporation_temp_scale, 0.0, 1.0)
/// ```
///
/// The clamp ensures the value stays in `[0, evaporation_rate]`.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `height_level` — per-tile discrete height level from the
///   tectonics stage.
/// * `temperature` — per-tile temperature from step 1.
/// * `params` — climate tuning parameters.
pub(crate) fn build_evaporation_field(
    bounds: MapBounds,
    height_level: &Field<u8>,
    temperature: &Field<f32>,
    params: &ClimateParams,
) -> Field<f32> {
    Field::from_fn(bounds, |coord| {
        if height_level[coord] > MAX_WATER_LEVEL {
            return 0.0;
        }
        let temp = temperature[coord];
        let scaled = (temp - params.evaporation_temp_floor) * params.evaporation_temp_scale;
        params.evaporation_rate * scaled.clamp(0.0, 1.0)
    })
}

/// Builds the per-tile coastal moisture source field.
///
/// Each land tile receives `coastal_moisture_factor` moisture per adjacent
/// water tile per iteration, modelling sea breeze and coastal humidity
/// independently of wind direction. Water tiles and tiles with no water
/// neighbours return 0.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `height_level` — per-tile discrete height level from the tectonics stage.
/// * `params` — climate tuning parameters.
// usize water-neighbour count is small (≤ 6); f32 precision is fine.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn build_coastal_source_field(
    bounds: MapBounds,
    height_level: &Field<u8>,
    params: &ClimateParams,
) -> Field<f32> {
    if params.coastal_moisture_factor == 0.0 {
        return Field::filled(bounds, 0.0);
    }
    Field::from_fn(bounds, |coord: Coord| {
        if height_level[coord] <= MAX_WATER_LEVEL {
            return 0.0;
        }
        let water_neighbours = bounds
            .neighbors(coord)
            .iter()
            .filter_map(|&nb| nb)
            .filter(|&nb| height_level[nb] <= MAX_WATER_LEVEL)
            .count();
        params.coastal_moisture_factor * water_neighbours as f32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    fn small_bounds() -> MapBounds {
        MapBounds::new(4, 4)
    }

    #[test]
    fn land_tiles_have_zero_evaporation() {
        let bounds = small_bounds();
        let height_level = Field::filled(bounds, 5_u8); // mountain
        let temperature = Field::filled(bounds, 1.0_f32);
        let params = ClimateParams::defaults();
        let evap = build_evaporation_field(bounds, &height_level, &temperature, &params);
        for &v in &evap {
            assert!(v.abs() < 1e-9_f32, "land tiles must have zero evaporation");
        }
    }

    #[test]
    fn ocean_tiles_evaporate_at_warm_temperature() {
        let bounds = small_bounds();
        let height_level = Field::filled(bounds, 0_u8); // ocean
        let temperature = Field::filled(bounds, 1.0_f32); // warm
        let params = ClimateParams::defaults();
        let evap = build_evaporation_field(bounds, &height_level, &temperature, &params);
        for &v in &evap {
            assert!(v > 0.0, "warm ocean tiles must evaporate");
        }
    }

    #[test]
    fn coast_tiles_are_moisture_sources() {
        let bounds = small_bounds();
        let height_level = Field::filled(bounds, 1_u8); // coast
        let temperature = Field::filled(bounds, 1.0_f32);
        let params = ClimateParams::defaults();
        let evap = build_evaporation_field(bounds, &height_level, &temperature, &params);
        for &v in &evap {
            assert!(v > 0.0, "coast tiles must be moisture sources");
        }
    }

    #[test]
    fn cold_water_tiles_below_floor_have_zero_evaporation() {
        let bounds = small_bounds();
        let height_level = Field::filled(bounds, 0_u8);
        // Temperature well below the floor.
        let temperature = Field::filled(bounds, -1.0_f32);
        let params = ClimateParams::defaults();
        let evap = build_evaporation_field(bounds, &height_level, &temperature, &params);
        for &v in &evap {
            assert!(
                v.abs() < 1e-9_f32,
                "cold water below temp_floor must have zero evaporation"
            );
        }
    }

    #[test]
    fn evaporation_capped_at_rate() {
        let bounds = small_bounds();
        let height_level = Field::filled(bounds, 0_u8);
        // Very high temperature — should still be capped at evaporation_rate.
        let temperature = Field::filled(bounds, 100.0_f32);
        let params = ClimateParams::defaults();
        let evap = build_evaporation_field(bounds, &height_level, &temperature, &params);
        for &v in &evap {
            assert!(
                (v - params.evaporation_rate).abs() < 1e-6,
                "evaporation must be capped at rate={}, got {v}",
                params.evaporation_rate
            );
        }
    }
}
