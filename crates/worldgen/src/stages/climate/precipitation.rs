//! Precipitation step kernel (step 5).
//!
//! Runs after each advection step. For each tile, subtracts precipitated
//! moisture from the moisture field and accumulates it into the running
//! precipitation total.
//!
//! # Precipitation fraction
//!
//! The fraction of a tile's moisture that precipitates per iteration is:
//!
//! ```text
//! capacity   = σ((temperature − temp_capacity_offset) × temp_capacity_scale)
//! orog_boost = 1 + orographic_factor × max(0, elevation − upwind_elevation)
//! fraction   = min(max_precipitation_fraction,
//!                  base_precipitation_rate × capacity × orog_boost)
//! precip     = fraction × moisture
//! ```
//!
//! where `σ(x) = 1 / (1 + e^{−x})` is the logistic sigmoid and
//! "upwind elevation" is the elevation of the neighbour from which wind
//! is blowing into this tile.
//!
//! # Separate pass
//!
//! Precipitation is computed as a separate sweep over the post-advection
//! field rather than inline during advection. This prevents the iteration
//! order from affecting which tiles see orographic enhancement first.

use coords::{Coord, Direction, MapBounds};

use crate::Field;
use crate::params::ClimateParams;
use crate::stages::climate::wind::HEX_UNIT_VECTORS;

// ------------- Public entry point -------------

/// Performs one precipitation step in place.
///
/// Subtracts precipitated moisture from `moisture` and accumulates the
/// removed amount into `precipitation_acc`.
///
/// # Arguments
///
/// * `moisture` — post-advection moisture field; modified in place.
/// * `precipitation_acc` — accumulated precipitation across all
///   iterations; incremented by this call.
/// * `temperature` — per-tile temperature field from step 1.
/// * `wind` — per-tile 2D wind vectors from step 2.
/// * `elevation` — continuous elevation field from the tectonics stage.
/// * `bounds` — map dimensions.
/// * `params` — climate tuning parameters.
pub(crate) fn precipitation_step(
    moisture: &mut Field<f32>,
    precipitation_acc: &mut Field<f32>,
    temperature: &Field<f32>,
    wind: &Field<[f32; 2]>,
    elevation: &Field<f32>,
    bounds: MapBounds,
    params: &ClimateParams,
) {
    let w = bounds.width().cast_signed();
    let h = bounds.height().cast_signed();

    for r in 0..h {
        for q in 0..w {
            let coord = Coord::new(q, r);
            let m = moisture[coord];
            if m <= 0.0 {
                continue;
            }

            let capacity = temp_capacity(temperature[coord], params);
            let orog = orographic_boost(coord, wind, elevation, bounds, params);
            let fraction = (params.base_precipitation_rate * capacity * orog)
                .min(params.max_precipitation_fraction)
                .max(0.0);
            let precip = fraction * m;

            *moisture.get_mut(coord).expect("coord is in bounds") -= precip;
            *precipitation_acc
                .get_mut(coord)
                .expect("coord is in bounds") += precip;
        }
    }
}

// ------------- Internal helpers -------------

/// Logistic sigmoid: σ(x) = 1 / (1 + e^{−x}).
///
/// Maps x ∈ (−∞, +∞) to (0, 1). Used for the temperature–capacity curve.
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Computes the temperature-dependent moisture-capacity factor.
///
/// Returns a value in `(0, 1)`: near 0 for cold tiles (air holds little
/// moisture), near 1 for warm tiles.
fn temp_capacity(temperature: f32, params: &ClimateParams) -> f32 {
    sigmoid((temperature - params.temp_capacity_offset) * params.temp_capacity_scale)
}

/// Computes the orographic precipitation boost for a tile.
///
/// Finds the upwind neighbour (the neighbour most opposed to the wind
/// direction) and returns `1 + orographic_factor × max(0, Δelev)` where
/// `Δelev = elevation[tile] − elevation[upwind]`.
///
/// Returns `1.0` if no upwind neighbour exists (polar tiles) or if the
/// tile is lower than its upwind neighbour.
fn orographic_boost(
    coord: Coord,
    wind: &Field<[f32; 2]>,
    elevation: &Field<f32>,
    bounds: MapBounds,
    params: &ClimateParams,
) -> f32 {
    let [wx, wy] = wind[coord];
    let elev = elevation[coord];

    // Find the upwind neighbour: the one whose unit vector is most
    // opposed to the wind (smallest dot product with wind vector).
    let mut min_proj = f32::INFINITY;
    let mut upwind_elev = elev; // default: no elevation gain

    for (i, dir) in Direction::ALL.iter().enumerate() {
        let [ux, uy] = HEX_UNIT_VECTORS[i];
        let proj = wx * ux + wy * uy;
        if proj < min_proj
            && let Some(neighbour) = bounds.neighbor(coord, *dir)
        {
            min_proj = proj;
            upwind_elev = elevation[neighbour];
        }
    }

    let delta_elev = (elev - upwind_elev).max(0.0);
    1.0 + params.orographic_factor * delta_elev
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    type TestFields = (
        Field<f32>,
        Field<f32>,
        Field<f32>,
        Field<[f32; 2]>,
        Field<f32>,
    );

    fn make_fields(
        bounds: MapBounds,
        moisture_val: f32,
        temp_val: f32,
        elev_val: f32,
    ) -> TestFields {
        (
            Field::filled(bounds, moisture_val),
            Field::filled(bounds, 0.0_f32),
            Field::filled(bounds, temp_val),
            Field::filled(bounds, [0.0_f32, 0.0]),
            Field::filled(bounds, elev_val),
        )
    }

    #[test]
    fn precipitation_reduces_moisture() {
        let bounds = MapBounds::new(5, 5);
        let (mut moisture, mut precip_acc, temperature, wind, elevation) =
            make_fields(bounds, 1.0, 2.0, 0.0); // warm, flat
        let params = ClimateParams::defaults();

        precipitation_step(
            &mut moisture,
            &mut precip_acc,
            &temperature,
            &wind,
            &elevation,
            bounds,
            &params,
        );

        // Some moisture should have precipitated.
        let total_moisture: f32 = moisture.iter().copied().sum();
        let total_precip: f32 = precip_acc.iter().copied().sum();
        assert!(
            total_moisture < 25.0,
            "some moisture should have precipitated"
        );
        assert!(total_precip > 0.0, "precipitation should have accumulated");

        // Conservation: moisture + precipitation ≈ initial.
        let initial_total = 25.0_f32; // 5×5 tiles, each with 1.0
        assert!(
            (total_moisture + total_precip - initial_total).abs() < 1e-4,
            "moisture + precipitation must equal initial"
        );
    }

    #[test]
    fn cold_tiles_precipitate_less_than_warm_tiles() {
        let bounds = MapBounds::new(4, 4);
        let params = ClimateParams::defaults();

        let (mut m_cold, mut p_cold, temp_cold, wind, elev) = make_fields(bounds, 1.0, -1.0, 0.0);
        precipitation_step(
            &mut m_cold,
            &mut p_cold,
            &temp_cold,
            &wind,
            &elev,
            bounds,
            &params,
        );

        let (mut m_warm, mut p_warm, temp_warm, wind2, elev2) = make_fields(bounds, 1.0, 2.0, 0.0);
        precipitation_step(
            &mut m_warm,
            &mut p_warm,
            &temp_warm,
            &wind2,
            &elev2,
            bounds,
            &params,
        );

        let cold_precip: f32 = p_cold.iter().copied().sum();
        let warm_precip: f32 = p_warm.iter().copied().sum();
        assert!(
            cold_precip < warm_precip,
            "cold tiles should precipitate less: cold={cold_precip}, warm={warm_precip}"
        );
    }

    #[test]
    fn zero_moisture_tiles_produce_no_precipitation() {
        let bounds = MapBounds::new(4, 4);
        let (mut moisture, mut precip_acc, temperature, wind, elevation) =
            make_fields(bounds, 0.0, 1.0, 0.0);
        let params = ClimateParams::defaults();

        precipitation_step(
            &mut moisture,
            &mut precip_acc,
            &temperature,
            &wind,
            &elevation,
            bounds,
            &params,
        );

        let total_precip: f32 = precip_acc.iter().copied().sum();
        assert!(
            total_precip.abs() < 1e-9_f32,
            "zero moisture must produce zero precipitation"
        );
    }

    #[test]
    fn sigmoid_zero_input_returns_half() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sigmoid_large_positive_approaches_one() {
        assert!(sigmoid(20.0) > 0.99);
    }

    #[test]
    fn sigmoid_large_negative_approaches_zero() {
        assert!(sigmoid(-20.0) < 0.01);
    }
}
