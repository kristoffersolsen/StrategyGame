//! Continuous elevation field construction (steps 6–7).
//!
//! Step 6 combines three contributions to build the raw elevation field:
//! 1. **Per-plate base** — a constant determined by [`PlateType`]
//!    (`continental_base_elevation` or `oceanic_base_elevation`).
//! 2. **Intra-plate noise** — low-amplitude wrap-aware `OpenSimplex` noise
//!    that breaks up the flat plate interiors without overwhelming the
//!    plate structure.
//! 3. **Boundary effects** — each boundary tile receives an elevation
//!    modifier derived from the `(own plate type, adjacent plate type,
//!    boundary type)` lookup table. Optional coherent source-strength
//!    noise breaks continuous ranges into chains and gaps, optional
//!    coherent sigma noise varies the local falloff width, and the
//!    resulting modifier is propagated outward via a Gaussian falloff
//!    (standard deviation `boundary_falloff_sigma` tiles before local
//!    sigma modulation).
//!
//! Step 7 adds a multi-octave detail-noise overlay uniformly over the
//! elevation field to introduce fine-grained topographic texture.
//!
//! # Wrap-awareness
//!
//! All noise evaluations use torus lifting: the `q` axis is mapped
//! onto a circle before being passed to the noise function so the field
//! is seamless at the east-west boundary.
//!
//! # Noise seeds
//!
//! The noise seeds are supplied by `tectonics::run` so this module is
//! pure-function (no RNG state). Boundary-effect noise uses a derived
//! seed and is a no-op unless its amplitudes are non-zero.

use std::f64::consts::TAU;

use coords::{Coord, MapBounds};
use noise::{NoiseFn, OpenSimplex};

use crate::Field;
use crate::params::TectonicsParams;
use crate::world_data::{BoundaryType, Plate, PlateId, PlateType};

// ------------- Torus-wrapping constants -------------

/// Torus radius for wrap-aware noise input on the q-axis.
///
/// Maps q ∈ `[0, width)` onto a circle of this radius. Setting it to
/// `1/(2π)` makes the circumference equal to 1.0, so adjacent q values
/// are separated by `1/width` on the circle — comparable to the r step.
const TORUS_RADIUS: f64 = 1.0 / (2.0 * std::f64::consts::PI);

/// Scale applied to the r coordinate before passing it to the noise function.
///
/// Matches the arc-step between adjacent q tiles on the torus so that
/// the noise varies at similar spatial frequency in both axes.
const NOISE_Z_SCALE: f64 = 1.0 / 150.0;

/// Lower clamp for local boundary falloff width.
const BOUNDARY_SIGMA_MIN_SCALE: f32 = 0.35;

/// Upper clamp for local boundary falloff width.
const BOUNDARY_SIGMA_MAX_SCALE: f32 = 1.75;

// ------------- Public entry point -------------

/// Seeds for each noise field used by elevation construction.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ElevationNoiseSeeds {
    /// `OpenSimplex` seed for intra-plate noise.
    pub(crate) intra: u32,
    /// `OpenSimplex` seed for boundary strength/sigma noise.
    pub(crate) boundary: u32,
    /// `OpenSimplex` seed for detail-noise octaves.
    pub(crate) detail: u32,
}

/// Builds the continuous elevation field by combining plate bases,
/// intra-plate noise, boundary effects, and multi-octave detail noise.
///
/// Step 6 populates the field; step 7 adds the detail-noise overlay.
/// The returned field is raw (not sea-level corrected) — normalisation
/// happens in `sea_level::quantize_elevation`.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `plate_id` — per-tile plate assignment from step 2–3.
/// * `plates` — per-plate properties from step 4.
/// * `seeds` — plate seed coordinates from step 1.
/// * `boundary_type` — per-tile boundary classification from step 5.
/// * `params` — tectonics tuning parameters.
/// * `noise_seeds` — deterministic seeds for the elevation noise fields.
pub(crate) fn build_elevation_field(
    bounds: MapBounds,
    plate_id: &Field<PlateId>,
    plates: &[Plate],
    boundary_type: &Field<Option<BoundaryType>>,
    params: &TectonicsParams,
    noise_seeds: ElevationNoiseSeeds,
) -> Field<f32> {
    let width_f64 = f64::from(bounds.width());

    // Step 6a: per-tile plate base + intra-plate noise.
    let intra_noise = OpenSimplex::new(noise_seeds.intra);
    let freq = f64::from(params.intra_plate_noise_frequency);

    let mut elevation = Field::from_fn(bounds, |tile| {
        let plate_type = plates[plate_id[tile] as usize].plate_type;
        let base = match plate_type {
            PlateType::Continental => params.continental_base_elevation,
            PlateType::Oceanic => params.oceanic_base_elevation,
        };
        let nx = noise_input(tile, width_f64, freq);
        #[allow(clippy::cast_possible_truncation)]
        let noise_val = intra_noise.get(nx) as f32 * params.intra_plate_noise_amplitude;
        base + noise_val
    });

    // Step 6b: propagate boundary elevation modifiers outward.
    add_boundary_effects(
        &mut elevation,
        bounds,
        plate_id,
        plates,
        boundary_type,
        params,
        noise_seeds.boundary,
    );

    // Step 7: multi-octave detail-noise overlay.
    add_detail_noise(
        &mut elevation,
        bounds,
        params,
        noise_seeds.detail,
        width_f64,
    );

    elevation
}

// ------------- Boundary-effect propagation -------------

/// Adds the Gaussian-propagated boundary elevation contributions to `elevation`.
///
/// For each boundary tile, looks up the elevation modifier from the
/// `(own plate type, adjacent plate type, boundary type)` table, then
/// accumulates `modifier × gaussian(hex_distance, sigma)` into every
/// tile within 3σ.
fn add_boundary_effects(
    elevation: &mut Field<f32>,
    bounds: MapBounds,
    plate_id: &Field<PlateId>,
    plates: &[Plate],
    boundary_type: &Field<Option<BoundaryType>>,
    params: &TectonicsParams,
    noise_seed: u32,
) {
    let base_sigma = params.boundary_falloff_sigma;
    let width_f64 = f64::from(bounds.width());
    let strength_noise =
        (params.boundary_strength_noise_amplitude > 0.0).then(|| OpenSimplex::new(noise_seed));
    let sigma_noise = (params.boundary_sigma_noise_amplitude > 0.0)
        .then(|| OpenSimplex::new(noise_seed.wrapping_add(0x9E37_79B9)));
    let max_sigma_scale = if params.boundary_sigma_noise_amplitude > 0.0 {
        (1.0 + params.boundary_sigma_noise_amplitude).min(BOUNDARY_SIGMA_MAX_SCALE)
    } else {
        1.0
    };
    let max_sigma = base_sigma * max_sigma_scale;
    #[allow(clippy::cast_possible_truncation)]
    let max_radius = (3.0 * max_sigma).ceil() as i32;

    for (source, &opt_bt) in boundary_type.iter_coords() {
        let Some(bt) = opt_bt else { continue };

        // Sum modifiers from all neighboring plates of a different type.
        let my_plate_id = plate_id[source];
        let my_type = plates[my_plate_id as usize].plate_type;

        let mut total_modifier = 0.0_f32;
        for neighbor in bounds.neighbors(source).iter().flatten() {
            let adj_id = plate_id[*neighbor];
            if adj_id != my_plate_id {
                let adj_type = plates[adj_id as usize].plate_type;
                total_modifier += elevation_modifier(my_type, adj_type, bt, params);
            }
        }

        if total_modifier == 0.0 {
            continue;
        }

        let strength_scale =
            boundary_strength_scale(source, width_f64, params, strength_noise.as_ref());
        if strength_scale == 0.0 {
            continue;
        }
        let local_sigma = boundary_sigma(source, width_f64, params, sigma_noise.as_ref());

        // Propagate modifier to all tiles within max_radius.
        let height = bounds.height().cast_signed();
        for dr in -max_radius..=max_radius {
            let target_r = source.r + dr;
            if target_r < 0 || target_r >= height {
                continue;
            }
            for dq in -max_radius..=max_radius {
                let target = Coord::new(source.q + dq, target_r);
                #[allow(clippy::cast_precision_loss)]
                let dist = bounds.distance(source, target) as f32;
                if dist > 3.0 * local_sigma {
                    continue;
                }
                let weight = gaussian(dist, local_sigma);
                if let Some(cell) = elevation.get_mut(target) {
                    *cell += total_modifier * strength_scale * weight;
                }
            }
        }
    }
}

/// Returns the elevation modifier for a boundary tile.
///
/// Lookup table: `(own plate type, adjacent plate type, boundary type)`.
/// Mixed-divergent boundaries (continental–oceanic or oceanic–continental
/// divergent) and all transform boundaries have negligible tectonic
/// elevation effects and return `0.0`.
fn elevation_modifier(
    own: PlateType,
    adjacent: PlateType,
    bt: BoundaryType,
    params: &TectonicsParams,
) -> f32 {
    match (own, adjacent, bt) {
        (PlateType::Continental, PlateType::Continental, BoundaryType::Convergent) => {
            params.mountain_elevation_boost
        }
        (PlateType::Continental, PlateType::Oceanic, BoundaryType::Convergent) => {
            params.coastal_mountain_boost
        }
        (PlateType::Oceanic, PlateType::Continental, BoundaryType::Convergent) => {
            params.subduction_trench_penalty
        }
        (PlateType::Oceanic, PlateType::Oceanic, BoundaryType::Convergent) => {
            params.island_arc_boost
        }
        (PlateType::Continental, PlateType::Continental, BoundaryType::Divergent) => {
            params.rift_valley_penalty
        }
        (PlateType::Oceanic, PlateType::Oceanic, BoundaryType::Divergent) => {
            params.mid_ocean_ridge_boost
        }
        // Mixed-divergent and all transform: no significant elevation effect.
        _ => 0.0,
    }
}

/// Gaussian weight for a tile at `distance` tiles from a boundary source.
fn gaussian(distance: f32, sigma: f32) -> f32 {
    let exponent = -(distance * distance) / (2.0 * sigma * sigma);
    exponent.exp()
}

/// Coherent multiplier for boundary source strength.
///
/// With amplitude zero this returns exactly `1.0`. Non-zero amplitudes
/// blend from the original uniform contribution toward a smooth mask
/// sampled at the boundary source tile.
fn boundary_strength_scale(
    source: Coord,
    width_f64: f64,
    params: &TectonicsParams,
    noise: Option<&OpenSimplex>,
) -> f32 {
    let amplitude = params.boundary_strength_noise_amplitude;
    let Some(noise) = noise.filter(|_| amplitude > 0.0) else {
        return 1.0;
    };

    let sample = normalized_noise_sample(
        noise,
        source,
        width_f64,
        f64::from(params.boundary_strength_noise_frequency),
    );
    let mask = smoothstep(0.25, 0.85, sample);
    1.0 - amplitude + amplitude * mask
}

/// Local Gaussian sigma for one boundary source tile.
///
/// With amplitude zero this returns exactly `boundary_falloff_sigma`.
/// Non-zero amplitudes vary the width coherently per source tile and
/// clamp the result to keep propagation radius bounded.
fn boundary_sigma(
    source: Coord,
    width_f64: f64,
    params: &TectonicsParams,
    noise: Option<&OpenSimplex>,
) -> f32 {
    let amplitude = params.boundary_sigma_noise_amplitude;
    let Some(noise) = noise.filter(|_| amplitude > 0.0) else {
        return params.boundary_falloff_sigma;
    };

    let sample = signed_noise_sample(
        noise,
        source,
        width_f64,
        f64::from(params.boundary_sigma_noise_frequency),
    );
    let scale =
        (1.0 + amplitude * sample).clamp(BOUNDARY_SIGMA_MIN_SCALE, BOUNDARY_SIGMA_MAX_SCALE);
    params.boundary_falloff_sigma * scale
}

/// Samples coherent noise and clamps it to `OpenSimplex`'s expected
/// `[-1, 1]` range before returning it as `f32`.
fn signed_noise_sample(noise: &OpenSimplex, tile: Coord, width_f64: f64, frequency: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let sample = noise.get(noise_input(tile, width_f64, frequency)) as f32;
    sample.clamp(-1.0, 1.0)
}

/// Samples coherent noise and remaps it to `[0, 1]`.
fn normalized_noise_sample(
    noise: &OpenSimplex,
    tile: Coord,
    width_f64: f64,
    frequency: f64,
) -> f32 {
    (signed_noise_sample(noise, tile, width_f64, frequency) + 1.0) * 0.5
}

/// Cubic Hermite interpolation over `[edge0, edge1]`.
fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ------------- Detail noise (step 7) -------------

/// Adds multi-octave wrap-aware detail noise to `elevation`.
///
/// Each octave doubles the spatial frequency and halves the amplitude
/// (standard fBm). The cumulative amplitude is renormalised to
/// `detail_noise_amplitude` so the total overlay stays within budget
/// regardless of the octave count.
fn add_detail_noise(
    elevation: &mut Field<f32>,
    bounds: MapBounds,
    params: &TectonicsParams,
    seed: u32,
    width_f64: f64,
) {
    let octaves = params.detail_noise_octaves;
    let base_freq = f64::from(params.detail_noise_frequency);
    let amplitude = params.detail_noise_amplitude;

    // Total amplitude across all octaves (geometric series: 1 + 1/2 + 1/4 + …).
    let total_amplitude: f32 = (0..octaves).map(|o| 0.5_f32.powi(i32::from(o))).sum();

    // Compute the overlay as a separate field to avoid a simultaneous
    // immutable + mutable borrow on `elevation`.
    let overlay = Field::from_fn(bounds, |tile| {
        let mut value = 0.0_f32;
        let mut freq_mult = 1.0_f64;
        let mut oct_amplitude = 1.0_f32;

        for octave in 0..octaves {
            // Each octave uses a distinct seed to avoid correlated patterns.
            let oct_seed = seed.wrapping_add(u32::from(octave).wrapping_mul(0x9E37_79B9));
            let noise = OpenSimplex::new(oct_seed);
            let nx = noise_input(tile, width_f64, base_freq * freq_mult);
            #[allow(clippy::cast_possible_truncation)]
            let sample = noise.get(nx) as f32;
            value += sample * oct_amplitude;
            freq_mult *= 2.0;
            oct_amplitude *= 0.5;
        }

        (value / total_amplitude) * amplitude
    });

    for (cell, &delta) in elevation.iter_mut().zip(overlay.iter()) {
        *cell += delta;
    }
}

// ------------- Noise utilities -------------

/// Computes the 3D wrap-aware noise input for `tile` at the given
/// frequency multiplier. Lifts q onto a torus circle so the noise is
/// seamless at the east-west boundary.
fn noise_input(tile: Coord, width_f64: f64, frequency: f64) -> [f64; 3] {
    let q_f64 = f64::from(tile.q);
    let r_f64 = f64::from(tile.r);

    let theta = TAU * q_f64 / width_f64;
    let nx = theta.cos() * TORUS_RADIUS * frequency;
    let ny = theta.sin() * TORUS_RADIUS * frequency;
    let nz = r_f64 * NOISE_Z_SCALE * frequency;

    [nx, ny, nz]
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::Rng;

    use crate::params::TectonicsParams;
    use crate::rng::{MasterRng, StageId};
    use crate::stages::tectonics::{
        boundary::classify_boundaries, growth::grow_plates_organic,
        plates::assign_plate_properties, seeds::place_plate_seeds,
    };

    fn run_pipeline(seed: u64) -> Field<f32> {
        let bounds = MapBounds::new(60, 40);
        let params = TectonicsParams::defaults();
        let mut rng = MasterRng::new(seed).fork(StageId::Tectonics);

        let seeds = place_plate_seeds(bounds, &params, &mut rng);
        let plate_id = grow_plates_organic(bounds, &seeds, &params, &mut rng);

        let plates = assign_plate_properties(seeds.len(), &params, &mut rng);

        let boundary_type = classify_boundaries(
            bounds,
            &plate_id,
            &plates,
            &seeds,
            params.convergence_threshold,
        );

        let intra_seed: u32 = rng.next_u32();
        let detail_seed: u32 = rng.next_u32();
        let boundary_seed = intra_seed.rotate_left(13) ^ detail_seed.rotate_right(7) ^ 0xA511_E9B3;
        let noise_seeds = ElevationNoiseSeeds {
            intra: intra_seed,
            boundary: boundary_seed,
            detail: detail_seed,
        };
        build_elevation_field(
            bounds,
            &plate_id,
            &plates,
            &boundary_type,
            &params,
            noise_seeds,
        )
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_elevation() {
            let a = run_pipeline(42);
            let b = run_pipeline(42);
            // f32 exact comparison is valid: same RNG stream → byte-identical floats.
            assert_eq!(a.as_slice(), b.as_slice());
        }

        #[allow(clippy::float_cmp)]
        #[test]
        fn different_seeds_produce_different_elevations() {
            let a = run_pipeline(1);
            let b = run_pipeline(2);
            let differ = a.as_slice().iter().zip(b.as_slice()).any(|(x, y)| x != y);
            assert!(
                differ,
                "different seeds should produce different elevations"
            );
        }
    }

    // ------------- Structural properties -------------

    mod structure {
        use super::*;

        #[test]
        fn all_values_are_finite() {
            let elevation = run_pipeline(42);
            for &v in &elevation {
                assert!(v.is_finite(), "elevation contains non-finite value: {v}");
            }
        }

        #[test]
        fn elevation_has_meaningful_spread() {
            let elevation = run_pipeline(42);
            let min = elevation.iter().copied().fold(f32::INFINITY, f32::min);
            let max = elevation.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            // The range must be large enough for quantile-thresholding to
            // produce distinct height levels.
            assert!(
                max - min > 0.5,
                "elevation range {min:.3}..{max:.3} is too narrow",
            );
        }
    }

    // ------------- Gaussian helper -------------

    mod gaussian_fn {
        use super::*;

        #[allow(clippy::float_cmp)]
        #[test]
        fn gaussian_is_one_at_zero_distance() {
            assert_eq!(gaussian(0.0, 8.0), 1.0);
        }

        #[test]
        fn gaussian_decays_with_distance() {
            assert!(gaussian(8.0, 8.0) < gaussian(4.0, 8.0));
            assert!(gaussian(16.0, 8.0) < gaussian(8.0, 8.0));
        }

        #[test]
        fn gaussian_approaches_zero_at_three_sigma() {
            // At 3σ the Gaussian is < 0.012.
            assert!(gaussian(24.0, 8.0) < 0.012);
        }
    }

    // ------------- Elevation modifier table -------------

    mod modifier_table {
        use super::*;

        fn params() -> TectonicsParams {
            TectonicsParams::defaults()
        }

        #[allow(clippy::float_cmp)]
        #[test]
        fn con_con_convergent_returns_mountain_boost() {
            let p = params();
            assert_eq!(
                elevation_modifier(
                    PlateType::Continental,
                    PlateType::Continental,
                    BoundaryType::Convergent,
                    &p
                ),
                p.mountain_elevation_boost
            );
        }

        #[allow(clippy::float_cmp)]
        #[test]
        fn oce_con_convergent_returns_trench_penalty() {
            let p = params();
            let modifier = elevation_modifier(
                PlateType::Oceanic,
                PlateType::Continental,
                BoundaryType::Convergent,
                &p,
            );
            assert!(modifier < 0.0, "trench penalty should be negative");
            assert_eq!(modifier, p.subduction_trench_penalty);
        }

        #[allow(clippy::float_cmp)]
        #[test]
        fn transform_boundary_returns_zero() {
            let p = params();
            for (own, adj) in [
                (PlateType::Continental, PlateType::Continental),
                (PlateType::Continental, PlateType::Oceanic),
                (PlateType::Oceanic, PlateType::Continental),
                (PlateType::Oceanic, PlateType::Oceanic),
            ] {
                assert_eq!(
                    elevation_modifier(own, adj, BoundaryType::Transform, &p),
                    0.0,
                    "transform boundary should have zero modifier"
                );
            }
        }
    }
}
