//! Advection iteration driver (step 6).
//!
//! Runs the source → advection → precipitation loop for a fixed number
//! of iterations (determined by `ClimateParams::iteration_count`).
//!
//! A fixed iteration count (not convergence-checked) is used for
//! determinism: the output is a function of the iteration count and
//! all other parameters, with no dependence on a convergence criterion
//! that could vary across platforms (ADR-0004).
//!
//! The loop uses double buffering: `moisture_cur` and `moisture_next`
//! alternate roles each iteration so the advection step can read from
//! a stable input field while writing to a fresh output.

use coords::MapBounds;

use crate::Field;
use crate::params::ClimateParams;
use crate::stages::climate::{advection, precipitation};

/// Runs the full advection-precipitation loop.
///
/// Starts from zero moisture everywhere, adds evaporation and advects
/// for `iteration_count` iterations, and returns the final steady-state
/// moisture field and the accumulated precipitation.
///
/// # Arguments
///
/// * `wind` — per-tile wind vectors from step 2 (constant across iterations).
/// * `evaporation` — per-tile evaporation sources from step 3 (constant).
/// * `temperature` — per-tile temperature from step 1 (constant).
/// * `elevation` — continuous elevation from the tectonics stage (constant).
/// * `height_level` — discrete height level from the tectonics stage.
/// * `bounds` — map dimensions.
/// * `params` — climate tuning parameters.
///
/// # Returns
///
/// `(moisture, precipitation)` — the final moisture field and the total
/// precipitation accumulated over all iterations.
pub(crate) fn run_iteration_loop(
    wind: &Field<[f32; 2]>,
    evaporation: &Field<f32>,
    temperature: &Field<f32>,
    elevation: &Field<f32>,
    height_level: &Field<u8>,
    bounds: MapBounds,
    params: &ClimateParams,
) -> (Field<f32>, Field<f32>) {
    let iteration_count = params.iteration_count(bounds.width());
    let mut moisture_cur = Field::filled(bounds, 0.0_f32);
    let mut moisture_next = Field::filled(bounds, 0.0_f32);
    let mut precipitation = Field::filled(bounds, 0.0_f32);

    for _ in 0..iteration_count {
        // Advection: read moisture_cur, write moisture_next (pre-zeroed).
        advection::advection_step(
            &moisture_cur,
            &mut moisture_next,
            wind,
            evaporation,
            height_level,
            bounds,
            params,
        );

        // Precipitation: reduces moisture_next in place, accumulates into precipitation.
        precipitation::precipitation_step(
            &mut moisture_next,
            &mut precipitation,
            temperature,
            wind,
            elevation,
            bounds,
            params,
        );

        // Swap buffers: moisture_next becomes the source for the next iteration.
        std::mem::swap(&mut moisture_cur, &mut moisture_next);
        // Reset the (now old) moisture_cur back to zero for the next advection pass.
        moisture_next.fill(0.0);
    }

    (moisture_cur, precipitation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    #[test]
    fn loop_produces_nonzero_moisture_when_ocean_present() {
        let bounds = MapBounds::new(20, 15);
        let params = ClimateParams {
            // Reduce iterations for test speed.
            iteration_count_factor: 0.5,
            iteration_count_min: 10,
            ..ClimateParams::defaults()
        };

        let wind = Field::filled(bounds, [0.1_f32, 0.0]);
        let evaporation = Field::filled(bounds, params.evaporation_rate);
        let temperature = Field::filled(bounds, 1.0_f32);
        let elevation = Field::filled(bounds, -1.0_f32); // all ocean depth
        let height_level = Field::filled(bounds, 0_u8); // all ocean

        let (moisture, precip) = run_iteration_loop(
            &wind,
            &evaporation,
            &temperature,
            &elevation,
            &height_level,
            bounds,
            &params,
        );

        let total_moisture: f32 = moisture.iter().copied().sum();
        let total_precip: f32 = precip.iter().copied().sum();
        assert!(
            total_moisture > 0.0,
            "moisture should be non-zero with evaporation sources, got {total_moisture}"
        );
        assert!(
            total_precip > 0.0,
            "some precipitation should have occurred, got {total_precip}"
        );
    }

    #[test]
    fn loop_is_deterministic() {
        let bounds = MapBounds::new(10, 8);
        let params = ClimateParams {
            iteration_count_factor: 0.5,
            iteration_count_min: 5,
            ..ClimateParams::defaults()
        };
        let wind = Field::filled(bounds, [0.1_f32, 0.0]);
        let evap = Field::filled(bounds, 0.03_f32);
        let temp = Field::filled(bounds, 0.5_f32);
        let elev = Field::filled(bounds, -0.5_f32);
        let height_level = Field::filled(bounds, 0_u8);

        let (m1, p1) =
            run_iteration_loop(&wind, &evap, &temp, &elev, &height_level, bounds, &params);
        let (m2, p2) =
            run_iteration_loop(&wind, &evap, &temp, &elev, &height_level, bounds, &params);

        #[allow(clippy::float_cmp)]
        for (&a, &b) in m1.iter().zip(m2.iter()) {
            assert_eq!(a, b, "moisture must be bit-identical across runs");
        }
        #[allow(clippy::float_cmp)]
        for (&a, &b) in p1.iter().zip(p2.iter()) {
            assert_eq!(a, b, "precipitation must be bit-identical across runs");
        }
    }
}
