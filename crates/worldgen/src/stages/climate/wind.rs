//! Wind field construction (step 2).
//!
//! The wind field is a `Field<[f32; 2]>` storing `[east_component,
//! south_component]` in hex Cartesian space. The east component is the
//! dominant zonal wind; the south component comes entirely from noise.
//!
//! # Base profile
//!
//! The zonal (east-west) base wind is `u(φ) = −A·cos(3π·φ)` where `A`
//! is `wind_amplitude` and `φ ∈ [−1, +1]` is signed latitude. This
//! produces three climate bands per hemisphere: trade winds (equatorial),
//! westerlies (mid-latitude), and polar easterlies (high-latitude).
//!
//! # Noise perturbation
//!
//! Low-frequency `OpenSimplex` noise (amplitude `wind_noise_amplitude`,
//! spatial frequency `wind_noise_frequency`) is added independently to
//! both components. The noise uses the torus-lifting technique from the
//! tectonics elevation module so the field is seamless at the east-west
//! wrap boundary.
//!
//! # CFL constraint
//!
//! `wind_amplitude` is validated to be ≤ [`crate::params::CFL_MAX_WIND_AMPLITUDE`]
//! (= 0.25) in [`ClimateParams::validate`] before this function is
//! called. Individual wind magnitudes after noise addition may slightly
//! exceed this in the rare worst case; a debug assertion catches that.

use std::f64::consts::TAU;

use coords::{Coord, MapBounds};
use noise::{NoiseFn, OpenSimplex};

use crate::Field;
use crate::params::ClimateParams;

// ------------- Torus-lifting constants (mirrors tectonics/elevation.rs) -------------

/// Torus radius for wrap-aware noise on the q-axis.
///
/// Setting it to `1/(2π)` makes the circumference equal 1.0, so
/// adjacent `q` values are separated by `1/width` on the circle —
/// comparable to the step along `r`.
const TORUS_RADIUS: f64 = 1.0 / TAU;

/// Scale applied to the `r` coordinate before noise evaluation.
///
/// Matches the arc-step between adjacent `q` tiles on the torus so
/// that the noise varies at similar spatial frequency in both axes.
const NOISE_Z_SCALE: f64 = 1.0 / 150.0;

// ------------- Public entry point -------------

/// Builds the 2D wind field.
///
/// Returns a `Field<[f32; 2]>` where element `[0]` is the east
/// component and `[1]` is the south component, both in hex Cartesian
/// space.
///
/// The noise seeds are drawn from the caller before entering this
/// function so that the function itself is deterministic given the
/// same seeds.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `params` — climate tuning parameters; `wind_amplitude`,
///   `wind_noise_amplitude`, and `wind_noise_frequency` are consumed.
/// * `noise_seed_x` — `OpenSimplex` seed for the east-component noise.
/// * `noise_seed_y` — `OpenSimplex` seed for the south-component noise.
pub(crate) fn build_wind_field(
    bounds: MapBounds,
    params: &ClimateParams,
    noise_seed_x: u32,
    noise_seed_y: u32,
) -> Field<[f32; 2]> {
    let noise_x = OpenSimplex::new(noise_seed_x);
    let noise_y = OpenSimplex::new(noise_seed_y);

    let height = bounds.height();
    let height_minus_one = f64::from((height.saturating_sub(1)).max(1));
    let width = f64::from(bounds.width());

    Field::from_fn(bounds, |coord| {
        let wind = sample_wind_at(coord, height_minus_one, width, params, &noise_x, &noise_y);
        debug_assert!(
            wind_magnitude(wind) <= 0.5,
            "wind magnitude {} exceeds CFL safety limit 0.5 at {:?}",
            wind_magnitude(wind),
            coord
        );
        wind
    })
}

/// Samples the wind vector at a single tile.
fn sample_wind_at(
    coord: Coord,
    height_minus_one: f64,
    map_width: f64,
    params: &ClimateParams,
    noise_x: &OpenSimplex,
    noise_y: &OpenSimplex,
) -> [f32; 2] {
    // Signed latitude φ ∈ [−1, +1]: −1 = north pole, +1 = south pole.
    let phi = 2.0 * f64::from(coord.r) / height_minus_one - 1.0;

    // Zonal base wind: u(φ) = −A·cos(3π·φ).
    let base_u = -f64::from(params.wind_amplitude) * (3.0 * std::f64::consts::PI * phi).cos();

    // Torus-lifted noise coordinates for wrap-seamless east-west noise.
    let angle = TAU * f64::from(coord.q) / map_width;
    let nx = TORUS_RADIUS * angle.cos() * f64::from(params.wind_noise_frequency);
    let ny = TORUS_RADIUS * angle.sin() * f64::from(params.wind_noise_frequency);
    let nz = f64::from(coord.r) * NOISE_Z_SCALE * f64::from(params.wind_noise_frequency);

    let perturb_x = noise_x.get([nx, ny, nz]) * f64::from(params.wind_noise_amplitude);
    let perturb_y = noise_y.get([nx, ny, nz]) * f64::from(params.wind_noise_amplitude);

    #[allow(clippy::cast_possible_truncation)]
    [(base_u + perturb_x) as f32, perturb_y as f32]
}

fn wind_magnitude(wind: [f32; 2]) -> f32 {
    (wind[0] * wind[0] + wind[1] * wind[1]).sqrt()
}

// ------------- Six hex unit vectors in Cartesian space -------------

/// The six hex neighbour unit vectors in the hex Cartesian frame.
///
/// Computed from the axial-to-Cartesian formula for flat-top hexes:
/// `x = 3/2·q`, `y = √3/2·q + √3·r`. Each vector has magnitude √3
/// before normalisation. After normalisation the vectors are spaced
/// exactly 60° apart.
///
/// Order matches [`coords::Direction::ALL`]:
/// `East`, `West`, `NorthEast`, `NorthWest`, `SouthEast`, `SouthWest`.
///
/// Used by the advection kernel to project wind onto each outflow
/// direction.
pub(crate) const HEX_UNIT_VECTORS: [[f32; 2]; 6] = [
    // East (+1, 0): Cartesian (3/2, √3/2) / √3 = (√3/2, 1/2)
    [0.866_025_4, 0.5],
    // West (−1, 0): Cartesian (−3/2, −√3/2) / √3 = (−√3/2, −1/2)
    [-0.866_025_4, -0.5],
    // NorthEast (+1, −1): Cartesian (3/2, −√3/2) / √3 = (√3/2, −1/2)
    [0.866_025_4, -0.5],
    // NorthWest (0, −1): Cartesian (0, −√3) / √3 = (0, −1)
    [0.0, -1.0],
    // SouthEast (0, +1): Cartesian (0, √3) / √3 = (0, 1)
    [0.0, 1.0],
    // SouthWest (−1, +1): Cartesian (−3/2, √3/2) / √3 = (−√3/2, 1/2)
    [-0.866_025_4, 0.5],
];

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    #[test]
    fn wind_field_has_correct_bounds() {
        let bounds = MapBounds::new(20, 15);
        let field = build_wind_field(bounds, &ClimateParams::defaults(), 1, 2);
        assert_eq!(field.bounds(), bounds);
    }

    #[test]
    fn wind_field_is_deterministic_for_same_seeds() {
        let bounds = MapBounds::new(20, 15);
        let params = ClimateParams::defaults();
        let a = build_wind_field(bounds, &params, 42, 99);
        let b = build_wind_field(bounds, &params, 42, 99);
        #[allow(clippy::float_cmp)]
        for ((_, v_a), (_, v_b)) in a.iter_coords().zip(b.iter_coords()) {
            assert_eq!(v_a, v_b, "field must be deterministic");
        }
    }

    #[test]
    fn wind_field_differs_for_different_seeds() {
        let bounds = MapBounds::new(20, 15);
        let params = ClimateParams::defaults();
        let a = build_wind_field(bounds, &params, 1, 2);
        let b = build_wind_field(bounds, &params, 3, 4);
        #[allow(clippy::float_cmp)]
        let any_differ = a
            .iter_coords()
            .zip(b.iter_coords())
            .any(|((_, va), (_, vb))| va != vb);
        assert!(any_differ, "different seeds must produce different fields");
    }

    #[test]
    fn hex_unit_vectors_are_unit_length() {
        for v in HEX_UNIT_VECTORS {
            let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-5,
                "unit vector not normalised: {v:?}"
            );
        }
    }

    #[test]
    fn hex_unit_vectors_are_60_degrees_apart() {
        // All pairwise dot products between non-opposite vectors should be ±0.5.
        // Adjacent vectors (60° apart) have dot product cos(60°) = 0.5.
        // Opposite vectors (180° apart) have dot product −1.
        let pairs_adjacent = [
            (0, 2), // East, NorthEast
            (2, 3), // NorthEast, NorthWest
            (3, 1), // NorthWest, West
        ];
        for (i, j) in pairs_adjacent {
            let dot = HEX_UNIT_VECTORS[i][0] * HEX_UNIT_VECTORS[j][0]
                + HEX_UNIT_VECTORS[i][1] * HEX_UNIT_VECTORS[j][1];
            assert!(
                (dot - 0.5).abs() < 1e-5,
                "adjacent vectors {i} and {j}: dot={dot} (expected 0.5)"
            );
        }
    }
}
