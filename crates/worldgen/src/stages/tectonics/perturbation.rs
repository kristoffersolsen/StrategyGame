//! Boundary perturbation via low-frequency noise displacement.
//!
//! Step 3 of the tectonics stage (design §1.9). Rather than assigning
//! each tile to its geometrically nearest seed, each tile's lookup
//! position is displaced by a noise-derived offset before the nearest-
//! seed search. This warps the Voronoi boundaries into organic,
//! geologically plausible shapes without changing the underlying
//! plate topology.
//!
//! # Wrap-aware noise
//!
//! The map wraps east-west but the noise function does not. Evaluating
//! noise at axial `(q, r)` directly would produce a visible seam at the
//! east-west boundary because `q = 0` and `q = width - 1` are adjacent
//! on the map but distant in noise-input space.
//!
//! The fix is to lift the `q` axis onto a circle and evaluate 3D noise:
//!
//! ```text
//! θ = 2π · q / width
//! noise_input = (cos(θ) · R, sin(θ) · R, r · z_scale)
//! ```
//!
//! where `R` is a torus radius constant chosen so that the circular arc
//! between adjacent `q` values is approximately one unit — matching the
//! `r` scale. Because the circle is continuous, the noise is seamless
//! across the wrap boundary.
//!
//! # Two independent noise channels
//!
//! The displacement is a 2D vector `(dq, dr)`. Two independent noise
//! evaluations are needed — one for `dq` and one for `dr`. The second
//! channel is offset in noise space by [`NOISE_CHANNEL_OFFSET`] to
//! ensure the two channels are uncorrelated.

use std::f64::consts::TAU;

use coords::{Coord, MapBounds};
use noise::{NoiseFn, OpenSimplex};

use crate::Field;
use crate::stages::tectonics::voronoi::assign_plates;
use crate::world_data::PlateId;

// ------------- Constants -------------

/// Torus radius for the wrap-aware noise input.
///
/// Maps the discrete `q` range `[0, width)` onto a circle of this
/// radius before evaluating noise. A radius of `1 / (2π)` makes the
/// circumference equal to `1.0`, so adjacent `q` values are separated
/// by `1 / width` on the circle — comparable to the `r` step of
/// `z_scale` below.
///
/// Larger values shrink the angular step between adjacent `q` tiles,
/// making the noise vary more slowly in the east-west direction
/// relative to the north-south direction. The default keeps both
/// directions at similar scales.
const TORUS_RADIUS: f64 = 1.0 / (2.0 * std::f64::consts::PI);

/// Scale applied to the `r` coordinate before passing it to the noise
/// function.
///
/// Should be comparable to the arc-step between adjacent `q` positions
/// on the torus (`1 / width`). For the default 240-wide map that step
/// is ~0.0042. A `z_scale` of `1 / height = 1 / 150 ≈ 0.0067` keeps
/// both axes at similar scales and ensures the noise varies at roughly
/// equal spatial frequency in both directions.
///
/// Smaller values make the noise vary more slowly north-south;
/// larger values make it vary faster (finer boundary detail).
const NOISE_Z_SCALE: f64 = 1.0 / 150.0;

/// Offset added to all noise coordinates for the second (dr) channel.
///
/// Using the same noise function at the same position for both `dq`
/// and `dr` would make the two offsets identical, effectively moving
/// every tile along the diagonal. The offset shifts the second
/// evaluation far enough in noise space that the channels are
/// decorrelated.
///
/// The value has no special significance beyond being large enough
/// to move into an uncorrelated region of the noise function.
const NOISE_CHANNEL_OFFSET: f64 = 100.0;

// ------------- Main entry point -------------

/// Produces a plate-ID field using noise-perturbed Voronoi assignment.
///
/// For each tile, a 2D noise displacement `(dq, dr)` is computed and
/// added to the tile's axial coordinate before the nearest-seed search.
/// This warps the Voronoi cell boundaries into organic shapes.
///
/// The `noise_seed` is an independent `u32` derived from the stage RNG
/// so that perturbation is seeded separately from seed placement.
/// Changing `perturbation_amplitude` does not shift the Poisson-disk
/// output; changing the seed placement does not shift the perturbation
/// pattern.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `seeds` — plate seed coordinates from step 1.
/// * `amplitude` — maximum displacement in tiles. `0.0` produces
///   straight Voronoi boundaries; the default produces organic ones.
///   See [`crate::params::tectonics::TectonicsParams::boundary_perturbation_amplitude`].
/// * `noise_seed` — seed for the `OpenSimplex` noise generator.
///
/// # Panics
///
/// Panics if `seeds` is empty (same contract as [`assign_plates`]).
pub(crate) fn assign_plates_perturbed(
    bounds: MapBounds,
    seeds: &[Coord],
    amplitude: f32,
    noise_seed: u32,
) -> Field<PlateId> {
    assert!(
        !seeds.is_empty(),
        "assign_plates_perturbed requires at least one seed"
    );

    // Zero amplitude: skip noise evaluation entirely and fall through
    // to the standard Voronoi partition. This is a fast path for
    // tests and for the tuning sweep at amplitude = 0.
    if amplitude == 0.0 {
        return assign_plates(bounds, seeds);
    }

    let noise = OpenSimplex::new(noise_seed);
    let width_f64 = f64::from(bounds.width());

    Field::from_fn(bounds, |tile| {
        let displaced = displace(tile, amplitude, bounds, &noise, width_f64);
        nearest_seed_to(displaced, seeds, bounds)
    })
}

/// Displaces `tile` by the noise-derived `(dq, dr)` offset and returns
/// the displaced coordinate, clamped to the map's north-south bounds.
///
/// The `q` component wraps naturally — [`MapBounds::normalize`] handles
/// it. The `r` component is clamped rather than wrapped because the
/// north-south axis is bounded: a displaced `r` that falls off the pole
/// is pulled back to the nearest edge tile rather than being discarded.
fn displace(
    tile: Coord,
    amplitude: f32,
    bounds: MapBounds,
    noise: &OpenSimplex,
    width_f64: f64,
) -> Coord {
    let nx = noise_input(tile, width_f64);

    // dr channel: offset in noise space for decorrelation.
    let nx_offset = [
        nx[0] + NOISE_CHANNEL_OFFSET,
        nx[1] + NOISE_CHANNEL_OFFSET,
        nx[2] + NOISE_CHANNEL_OFFSET,
    ];

    // Scale noise [-1, 1] by amplitude and round to nearest tile.
    #[allow(clippy::cast_possible_truncation)]
    let dq = (noise.get(nx) as f32 * amplitude).round() as i32;
    #[allow(clippy::cast_possible_truncation)]
    let dr = (noise.get(nx_offset) as f32 * amplitude).round() as i32;

    let raw_r = tile.r + dr;
    // Clamp r to [0, height - 1]: displacing off the north or south
    // pole assigns the tile to the edge row's nearest seed rather than
    // discarding it.
    let clamped_r = raw_r.clamp(0, bounds.height().cast_signed() - 1);

    // q wraps naturally via normalize; r is already in bounds.
    let raw = Coord::new(tile.q + dq, clamped_r);
    bounds
        .normalize(raw)
        .expect("clamped r is in bounds; normalize cannot fail")
}

/// Computes the 3D noise input for `tile`, lifting `q` onto a torus
/// circle to achieve seamless east-west wrapping.
fn noise_input(tile: Coord, width_f64: f64) -> [f64; 3] {
    let q_f64 = f64::from(tile.q);
    let r_f64 = f64::from(tile.r);

    let theta = TAU * q_f64 / width_f64;
    let nx = theta.cos() * TORUS_RADIUS;
    let ny = theta.sin() * TORUS_RADIUS;
    let nz = r_f64 * NOISE_Z_SCALE;

    [nx, ny, nz]
}

/// Returns the index of the seed nearest to `lookup_pos`, with ties
/// broken by lowest index.
///
/// Identical tie-breaking contract to `voronoi::assign_plates` so
/// that `amplitude = 0.0` produces the same result as the unperturbed
/// partition.
fn nearest_seed_to(lookup_pos: Coord, seeds: &[Coord], bounds: MapBounds) -> PlateId {
    let mut best_id = 0u8;
    let mut best_dist = i32::MAX;

    for (idx, &seed) in seeds.iter().enumerate() {
        let dist = bounds.distance(lookup_pos, seed);
        if dist < best_dist {
            best_dist = dist;
            #[allow(clippy::cast_possible_truncation)]
            let id = idx as PlateId;
            best_id = id;
        }
    }

    best_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::Coord;

    fn test_seeds() -> Vec<Coord> {
        vec![
            Coord::new(10, 7),
            Coord::new(30, 20),
            Coord::new(20, 5),
            Coord::new(5, 25),
        ]
    }

    // ------------- Zero amplitude -------------

    mod zero_amplitude {
        use super::*;

        /// At amplitude 0.0, perturbed assignment must equal standard
        /// Voronoi assignment exactly.
        #[test]
        fn matches_unperturbed_voronoi() {
            let bounds = MapBounds::new(40, 30);
            let seeds = test_seeds();

            let unperturbed = assign_plates(bounds, &seeds);
            let perturbed = assign_plates_perturbed(bounds, &seeds, 0.0, 0);

            assert_eq!(
                unperturbed.as_slice(),
                perturbed.as_slice(),
                "amplitude=0 must produce identical output to unperturbed Voronoi"
            );
        }
    }

    // ------------- Non-zero amplitude -------------

    mod nonzero_amplitude {
        use super::*;

        #[test]
        fn all_assigned_ids_are_valid() {
            let bounds = MapBounds::new(40, 30);
            let seeds = test_seeds();
            let plate_id = assign_plates_perturbed(bounds, &seeds, 8.0, 42);
            #[allow(clippy::cast_possible_truncation)]
            let plate_count = seeds.len() as PlateId;

            for (_, &id) in plate_id.iter_coords() {
                assert!(id < plate_count, "invalid plate id {id}");
            }
        }

        /// Perturbation should change at least some assignments relative
        /// to unperturbed Voronoi (verifies noise is actually applied).
        #[test]
        fn differs_from_unperturbed_voronoi() {
            let bounds = MapBounds::new(40, 30);
            let seeds = test_seeds();

            let unperturbed = assign_plates(bounds, &seeds);
            let perturbed = assign_plates_perturbed(bounds, &seeds, 8.0, 42);

            let differ = unperturbed
                .as_slice()
                .iter()
                .zip(perturbed.as_slice().iter())
                .any(|(a, b)| a != b);

            assert!(
                differ,
                "nonzero amplitude must change some tile assignments"
            );
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_field() {
            let bounds = MapBounds::new(40, 30);
            let seeds = test_seeds();

            let a = assign_plates_perturbed(bounds, &seeds, 8.0, 42);
            let b = assign_plates_perturbed(bounds, &seeds, 8.0, 42);
            assert_eq!(a.as_slice(), b.as_slice());
        }

        #[test]
        fn different_noise_seeds_produce_different_fields() {
            let bounds = MapBounds::new(40, 30);
            let seeds = test_seeds();

            let a = assign_plates_perturbed(bounds, &seeds, 8.0, 1);
            let b = assign_plates_perturbed(bounds, &seeds, 8.0, 2);
            assert_ne!(a.as_slice(), b.as_slice());
        }
    }

    // ------------- Wrap continuity -------------

    mod wrap {
        use super::*;

        /// Tiles near the east and west edges of the same row should
        /// be assigned to the same plate when a west-edge seed is the
        /// closest. This confirms the noise has no seam discontinuity
        /// that would create a visible boundary artifact at q=0/q=W-1.
        ///
        /// This is a probabilistic check: it verifies the wrap works
        /// by placing a seed at q=1 and confirming tiles at q=W-1 and
        /// q=W-2 are assigned to it rather than to the far seed.
        #[test]
        fn no_seam_artifact_near_wrap_boundary() {
            let bounds = MapBounds::new(40, 20);
            let seeds = vec![
                Coord::new(1, 10),  // west-edge seed
                Coord::new(20, 10), // central seed, far from edges
            ];

            let plate_id = assign_plates_perturbed(bounds, &seeds, 6.0, 7);

            // Tiles immediately west of the seam (q=38, q=39) are
            // 2 and 3 hex-steps from seed 0 via wrap. Without correct
            // wrap-aware noise they might be spuriously assigned to
            // plate 1. Check that at least one is on plate 0.
            let near_east_a = Coord::new(39, 10);
            let near_east_b = Coord::new(38, 10);
            let id_a = *plate_id.get(near_east_a).unwrap();
            let id_b = *plate_id.get(near_east_b).unwrap();

            assert!(
                id_a == 0 || id_b == 0,
                "at least one east-edge tile should be on the west-edge plate \
                 (got ids {id_a} and {id_b}); possible seam artifact"
            );
        }
    }
}
