//! Latitude-based temperature field construction (step 1).
//!
//! Temperature is a linear interpolation between equator and pole values
//! based on absolute latitude, then adjusted downward by an elevation
//! lapse rate for tiles above sea level, and finally shifted by the
//! global `climate_band_offset`.
//!
//! The result is independent of moisture; it can be tested in isolation
//! before the advection loop is wired up.

use coords::{Coord, MapBounds};

use crate::Field;
use crate::params::ClimateParams;

/// Builds the temperature field from latitude and elevation.
///
/// For each tile at row `r` in a map of height `h`:
///
/// 1. Latitude `φ = 2·r / (h − 1) − 1` ∈ `[−1, +1]` (−1 = north
///    pole, +1 = south pole).
/// 2. Latitude temperature = linear interpolation from
///    `equator_temperature` (at `|φ| = 0`) to `pole_temperature`
///    (at `|φ| = 1`).
/// 3. Elevation correction: subtract `elevation_lapse_rate ×
///    max(0, elevation)` — only above sea level.
/// 4. Global offset: add `climate_band_offset`.
pub(crate) fn build_temperature_field(
    bounds: MapBounds,
    elevation: &Field<f32>,
    params: &ClimateParams,
) -> Field<f32> {
    let height = bounds.height();
    // height is always ≥ 1 (MapBounds rejects zero dimensions), but
    // use max(1, height-1) so single-row maps don't divide by zero.
    #[allow(clippy::cast_precision_loss)]
    let height_minus_one = (height.saturating_sub(1)).max(1) as f32;

    Field::from_fn(bounds, |coord| {
        let temp_lat = latitude_temperature(coord, height_minus_one, params);
        let elev = elevation[coord];
        let lapse = params.elevation_lapse_rate * elev.max(0.0);
        temp_lat - lapse + params.climate_band_offset
    })
}

/// Computes the latitude-based temperature before elevation and band
/// offset corrections.
fn latitude_temperature(coord: Coord, height_minus_one: f32, params: &ClimateParams) -> f32 {
    // φ ∈ [−1, +1]: −1 = north (r=0), +1 = south (r=height−1).
    #[allow(clippy::cast_precision_loss)]
    let phi = 2.0 * coord.r as f32 / height_minus_one - 1.0;
    // Absolute latitude ∈ [0, 1]: 0 at equator, 1 at poles.
    let abs_lat = phi.abs();
    // Linear interpolation from equator (abs_lat=0) to pole (abs_lat=1).
    params.equator_temperature + abs_lat * (params.pole_temperature - params.equator_temperature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    fn default_params() -> ClimateParams {
        ClimateParams::defaults()
    }

    #[test]
    fn equator_row_has_highest_temperature() {
        let bounds = MapBounds::new(10, 11); // odd height so row 5 is equator
        let elevation = Field::filled(bounds, 0.0_f32);
        let temp = build_temperature_field(bounds, &elevation, &default_params());

        // Row 5 is equator (φ=0): expect equator_temperature + offset.
        let equator_temp = *temp.get(coords::Coord::new(0, 5)).expect("in bounds");
        let expected = default_params().equator_temperature + default_params().climate_band_offset;
        assert!(
            (equator_temp - expected).abs() < 1e-5,
            "equator row should equal equator_temperature, got {equator_temp}"
        );
    }

    #[test]
    fn pole_rows_have_lowest_temperature() {
        let bounds = MapBounds::new(10, 11);
        let elevation = Field::filled(bounds, 0.0_f32);
        let params = default_params();
        let temp = build_temperature_field(bounds, &elevation, &params);

        let north_pole = *temp.get(coords::Coord::new(0, 0)).expect("in bounds");
        let south_pole = *temp.get(coords::Coord::new(0, 10)).expect("in bounds");
        let expected = params.pole_temperature + params.climate_band_offset;
        assert!(
            (north_pole - expected).abs() < 1e-5,
            "north pole: {north_pole}"
        );
        assert!(
            (south_pole - expected).abs() < 1e-5,
            "south pole: {south_pole}"
        );
    }

    #[test]
    fn temperature_decreases_monotonically_toward_poles() {
        let bounds = MapBounds::new(2, 21);
        let elevation = Field::filled(bounds, 0.0_f32);
        let temp = build_temperature_field(bounds, &elevation, &default_params());

        // Equator at row 10; temperature should decrease outward from it.
        let rows: Vec<f32> = (0..21_i32)
            .map(|r| *temp.get(coords::Coord::new(0, r)).expect("in bounds"))
            .collect();
        for r in 0..10 {
            assert!(
                rows[r] <= rows[r + 1],
                "temperature should increase toward equator at r={r}"
            );
        }
        for r in 10..20 {
            assert!(
                rows[r] >= rows[r + 1],
                "temperature should decrease away from equator at r={r}"
            );
        }
    }

    #[test]
    fn elevation_lapse_cools_high_tiles() {
        let bounds = MapBounds::new(2, 11);
        let mut elevation = Field::filled(bounds, 0.0_f32);
        // Column 1 has elevation = 1.0 (mountain-level).
        for r in 0..11_i32 {
            elevation.set(coords::Coord::new(1, r), 1.0);
        }
        let params = default_params();
        let temp = build_temperature_field(bounds, &elevation, &params);

        for r in 0..11_i32 {
            let flat = *temp.get(coords::Coord::new(0, r)).expect("in bounds");
            let high = *temp.get(coords::Coord::new(1, r)).expect("in bounds");
            let expected_drop = params.elevation_lapse_rate * 1.0;
            assert!(
                (flat - high - expected_drop).abs() < 1e-5,
                "elevation lapse wrong at r={r}: flat={flat}, high={high}"
            );
        }
    }

    #[test]
    fn climate_band_offset_shifts_all_tiles() {
        let bounds = MapBounds::new(5, 11);
        let elevation = Field::filled(bounds, 0.0_f32);
        let params_base = default_params();
        let params_shifted = ClimateParams {
            climate_band_offset: 0.3,
            ..default_params()
        };

        let temp_base = build_temperature_field(bounds, &elevation, &params_base);
        let temp_shifted = build_temperature_field(bounds, &elevation, &params_shifted);

        for (&base, &shifted) in temp_base.iter().zip(temp_shifted.iter()) {
            assert!(
                (shifted - base - 0.3).abs() < 1e-5,
                "offset should shift uniformly"
            );
        }
    }
}
