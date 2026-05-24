//! Plate seed placement via wrap-aware Poisson disk sampling.
//!
//! Step 1 of the tectonics stage (design §1.9). Produces an
//! evenly-spaced set of seed coordinates from which the Voronoi
//! partition (step 2) will derive plate territories.
//!
//! # Algorithm
//!
//! Bridson's 2007 Poisson-disk sampling, adapted for a cylindrical
//! hex grid:
//!
//! 1. Compute a minimum-spacing radius `r` from the target plate
//!    count and the map area so plate count is roughly invariant
//!    to map size.
//! 2. Place an initial seed at a deterministic random tile.
//! 3. Maintain an active list of seeds whose neighbourhood is not yet
//!    fully explored. For each active seed, attempt
//!    [`BRIDSON_CANDIDATE_COUNT`] candidate placements drawn from the
//!    annulus `[r, 2r]` around it; accept any that satisfies the
//!    minimum-distance constraint against *every* existing seed. If
//!    no candidate is accepted, the active seed is removed from the
//!    active list.
//! 4. Repeat until the active list is empty.
//!
//! All distance checks use [`MapBounds::distance`], which is
//! wrap-aware in the east-west direction and bounded in the
//! north-south direction. Candidates whose `r` falls outside the
//! map are rejected; candidates whose `q` is off-edge are normalised
//! into the canonical `[0, width)` range.

use coords::{Coord, MapBounds};
use rand::RngExt;
use std::f32::consts::TAU;

use crate::params::TectonicsParams;
use crate::rng::StageRng;

// ------------- Algorithm constants -------------

/// Number of candidate placements attempted per active seed before
/// the seed is removed from the active list.
///
/// Bridson recommends ~30; this is the canonical value across most
/// Poisson-disk implementations. Lower values produce sparser output
/// (active seeds give up too early); higher values are wasted work
/// once the map is mostly filled.
const BRIDSON_CANDIDATE_COUNT: u32 = 30;

/// Outer radius of the candidate annulus, as a multiple of the
/// minimum-spacing radius `r`.
///
/// Bridson's algorithm draws candidates from the annulus `[r, 2r]`:
/// closer than `r` and the candidate is always rejected; further than
/// `2r` and the algorithm leaves visible voids.
const BRIDSON_OUTER_RADIUS_MULTIPLIER: f32 = 2.0;

/// Empirical packing-efficiency factor for Bridson's algorithm on
/// this grid geometry.
///
/// The theoretical center-to-center spacing for perfect hexagonal
/// packing is `2 * cell_r`. In practice, the discrete hex grid,
/// integer distance rounding via `ceil`, and north-south boundary
/// rejections reduce packing efficiency below the continuous ideal.
/// This factor is calibrated so that the achieved seed count clusters
/// near `target_plate_count` across a range of seeds on the default
/// map size.
///
/// If this is changed, re-run the achieved-count test across at least
/// 20 seeds at targets 4, 8, 10, 16, and 24 to confirm the counts
/// remain within the expected window.
const PACKING_EFFICIENCY_FACTOR: f32 = 1.4;

// ------------- Radius derivation -------------

/// Returns the minimum-spacing radius (in hex distance units) for the
/// given target plate count and map bounds.
///
/// Derived from the equal-area approximation: treat each plate as a
/// disk of radius `cell_r` covering `1/N` of the map area:
///
/// ```text
/// N · π · cell_r² = W · H
/// cell_r = sqrt(W · H / (π · N))
/// ```
///
/// `cell_r` is the distance from a seed to its cell boundary. The
/// minimum distance *between two seeds* is `2 · cell_r` (center to
/// center of two adjacent cells), which is what the Poisson-disk
/// spacing constraint enforces. Returning `cell_r` instead would
/// halve the spacing and roughly quadruple the achieved seed count.
///
/// The Poisson-disk algorithm maintains hex-distance `≥ r` between
/// any two accepted seeds, so the achieved count clusters near the
/// target but can differ by a few.
///
/// # Panics
///
/// Panics if `target_plate_count == 0`; [`TectonicsParams::validate`]
/// rejects this at the parameter-validation gate, so the panic is a
/// programmer-error guard rather than a user-facing error path.
#[must_use]
pub(crate) fn poisson_disk_radius(target_plate_count: u8, bounds: MapBounds) -> f32 {
    assert!(
        target_plate_count > 0,
        "target_plate_count must be > 0; \
         TectonicsParams::validate rejects this earlier"
    );
    // Casts: u32 → f32 is lossy above 2^24, but map dimensions are
    // bounded by i32::MAX in MapBounds. The 240x150 default and even
    // 400x250 Huge variant are well below 2^24.
    #[allow(clippy::cast_precision_loss)]
    let area = (bounds.width() as f32) * (bounds.height() as f32);
    #[allow(clippy::cast_precision_loss)]
    let count = f32::from(target_plate_count);
    // cell_r is the Voronoi cell radius; seed-to-seed spacing is 2 * cell_r.
    let cell_r = (area / (std::f32::consts::PI * count)).sqrt();
    cell_r * PACKING_EFFICIENCY_FACTOR
}

// ------------- Initial seed -------------

/// Returns a deterministic uniformly-random in-bounds coordinate for
/// the initial Poisson-disk seed.
///
/// Drawing the initial seed from the stage RNG rather than fixing
/// it at `(0, 0)` avoids the north-west corner always being a plate
/// centre — a visible artifact when comparing seeds.
fn initial_seed(bounds: MapBounds, rng: &mut StageRng) -> Coord {
    // random_range over u32 ranges; cast_signed lossless because
    // MapBounds enforces width/height <= i32::MAX.
    let q = rng.random_range(0..bounds.width()).cast_signed();
    let r = rng.random_range(0..bounds.height()).cast_signed();
    Coord::new(q, r)
}

// ------------- Candidate generation -------------

/// Generates a random candidate coordinate in the annulus around
/// `centre`.
///
/// Draws an angle `θ ∈ [0, 2π)` and a distance `d ∈ [r, 2r]` (both
/// uniform), converts the Euclidean polar offset to an axial offset,
/// and returns the candidate after east-west wrap normalisation. The
/// caller is responsible for hex-distance validation: this function
/// only produces a candidate position, it does not check spacing.
///
/// Returns `None` if the candidate's `r` component falls outside the
/// map's north-south bounds (the bounded axis does not wrap).
///
/// # Note on the axial-as-Euclidean approximation
///
/// Axial coordinates are not isotropic — moving one step east and one
/// step north-east are both unit hex-distance moves but produce
/// different Euclidean displacements. Treating axial `(dq, dr)` as
/// Euclidean `(x, y)` is a small geometric inaccuracy that distorts
/// the *distribution* of candidates within the annulus. It does not
/// affect *correctness*: the caller validates spacing using
/// [`MapBounds::distance`] (the true hex metric), so accepted seeds
/// always satisfy the spacing constraint exactly.
fn random_candidate(
    centre: Coord,
    radius: f32,
    bounds: MapBounds,
    rng: &mut StageRng,
) -> Option<Coord> {
    // f64 here would be overkill; f32 angles are accurate to ~1e-7
    // radians, far below tile-level resolution.
    let angle: f32 = rng.random_range(0.0..TAU);
    let distance: f32 = rng.random_range(radius..(radius * BRIDSON_OUTER_RADIUS_MULTIPLIER));

    let (sin, cos) = angle.sin_cos();

    // Round to nearest integer. The candidate is a hex position, so
    // sub-tile precision in the offset is discarded.
    #[allow(clippy::cast_possible_truncation)]
    let dq = (distance * cos).round() as i32;
    #[allow(clippy::cast_possible_truncation)]
    let dr = (distance * sin).round() as i32;

    let raw = Coord::new(centre.q + dq, centre.r + dr);
    bounds.normalize(raw)
}

// ------------- Spacing check -------------

/// Returns `true` if `candidate` is at least `min_distance` hex-steps
/// from every coordinate in `existing`.
///
/// Linear in `existing.len()`. At the plate counts this stage
/// produces (≤ 32), this is faster in practice than maintaining a
/// background grid acceleration structure.
fn satisfies_spacing(
    candidate: Coord,
    existing: &[Coord],
    min_distance: i32,
    bounds: MapBounds,
) -> bool {
    existing
        .iter()
        .all(|&other| bounds.distance(candidate, other) >= min_distance)
}

// ------------- Main entry point -------------

/// Produces the set of plate seeds for the given parameters.
///
/// Output is deterministic in `(bounds, params, rng_state)`. Seed
/// order in the returned vector reflects the order they were
/// accepted by the Poisson-disk algorithm, which is itself a
/// deterministic function of the inputs.
///
/// The achieved seed count clusters near
/// [`TectonicsParams::target_plate_count`] but can differ by a few
/// in either direction; callers that need an exact count are
/// responsible for any post-processing.
pub(crate) fn place_plate_seeds(
    bounds: MapBounds,
    params: &TectonicsParams,
    rng: &mut StageRng,
) -> Vec<Coord> {
    let radius_f = poisson_disk_radius(params.target_plate_count, bounds);
    // The spacing constraint is a hex-distance integer; round up so
    // accepted seeds are at least `ceil(radius)` steps apart.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let min_distance = radius_f.ceil() as i32;

    let mut accepted: Vec<Coord> = Vec::new();
    let mut active: Vec<Coord> = Vec::new();

    let first = initial_seed(bounds, rng);
    accepted.push(first);
    active.push(first);

    while !active.is_empty() {
        // Pick a uniformly-random active seed. Removing it from the
        // active list (after all candidate attempts fail) is the
        // termination condition; swap_remove is fine because we
        // don't care about active-list order.
        // Cast: active.len() always > 0 inside the loop. usize → u32
        // is safe at the plate counts we deal with.
        let active_idx = rng.random_range(0..active.len());
        let centre = active[active_idx];

        let mut found = false;
        for _ in 0..BRIDSON_CANDIDATE_COUNT {
            let Some(candidate) = random_candidate(centre, radius_f, bounds, rng) else {
                // Candidate fell off the north/south edge; skip and
                // try another. We do *not* consume an extra attempt
                // beyond the loop count: this attempt counts.
                continue;
            };
            if satisfies_spacing(candidate, &accepted, min_distance, bounds) {
                accepted.push(candidate);
                active.push(candidate);
                found = true;
                break;
            }
        }

        if !found {
            active.swap_remove(active_idx);
        }
    }

    accepted
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{MasterRng, StageId};

    fn rng(seed: u64) -> StageRng {
        MasterRng::new(seed).fork(StageId::Tectonics)
    }

    // ------------- Radius derivation -------------

    mod radius {
        use super::*;

        #[test]
        fn radius_decreases_as_plate_count_grows() {
            let bounds = MapBounds::new(240, 150);
            let r_few = poisson_disk_radius(4, bounds);
            let r_many = poisson_disk_radius(20, bounds);
            assert!(
                r_few > r_many,
                "more plates must produce smaller radius (got {r_few} vs {r_many})"
            );
        }

        #[test]
        fn radius_increases_with_map_area() {
            let r_small = poisson_disk_radius(10, MapBounds::new(120, 80));
            let r_large = poisson_disk_radius(10, MapBounds::new(320, 200));
            assert!(
                r_large > r_small,
                "larger map must produce larger radius (got {r_small} vs {r_large})"
            );
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_output() {
            let bounds = MapBounds::new(240, 150);
            let params = TectonicsParams::defaults();

            let a = place_plate_seeds(bounds, &params, &mut rng(42));
            let b = place_plate_seeds(bounds, &params, &mut rng(42));
            assert_eq!(a, b);
        }

        #[test]
        fn different_seeds_produce_different_output() {
            let bounds = MapBounds::new(240, 150);
            let params = TectonicsParams::defaults();

            let a = place_plate_seeds(bounds, &params, &mut rng(1));
            let b = place_plate_seeds(bounds, &params, &mut rng(2));
            // Vanishing chance of collision; if this ever fires for a
            // legitimate reason, pick different seed values.
            assert_ne!(a, b);
        }
    }

    // ------------- Spacing invariant -------------

    mod spacing {
        use super::*;

        /// The defining invariant: every pair of accepted seeds is
        /// at least `ceil(radius)` hex-steps apart.
        #[test]
        fn all_pairs_satisfy_minimum_spacing() {
            let bounds = MapBounds::new(240, 150);
            let params = TectonicsParams::defaults();
            let seeds = place_plate_seeds(bounds, &params, &mut rng(0));

            let radius = poisson_disk_radius(params.target_plate_count, bounds);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let min_d = radius.ceil() as i32;

            for (i, &a) in seeds.iter().enumerate() {
                for &b in &seeds[i + 1..] {
                    let d = bounds.distance(a, b);
                    assert!(
                        d >= min_d,
                        "seeds {a:?} and {b:?} are {d} apart, expected >= {min_d}"
                    );
                }
            }
        }

        #[test]
        fn spacing_invariant_holds_across_many_seeds() {
            let bounds = MapBounds::new(240, 150);
            let params = TectonicsParams::defaults();
            let radius = poisson_disk_radius(params.target_plate_count, bounds);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let min_d = radius.ceil() as i32;

            for seed in 0..32_u64 {
                let seeds = place_plate_seeds(bounds, &params, &mut rng(seed));
                for (i, &a) in seeds.iter().enumerate() {
                    for &b in &seeds[i + 1..] {
                        assert!(bounds.distance(a, b) >= min_d, "seed {seed}");
                    }
                }
            }
        }
    }

    // ------------- Achieved count -------------

    mod achieved_count {
        use super::*;

        /// The achieved count should cluster near the target. We test
        /// a generous window around the target to confirm the
        /// algorithm produces approximately the requested plate count.
        #[test]
        fn achieved_count_is_close_to_target() {
            let bounds = MapBounds::new(240, 150);
            for target in [4_u8, 8, 10, 16, 24] {
                let params = TectonicsParams {
                    target_plate_count: target,
                    ..TectonicsParams::defaults()
                };
                for seed in 0..8_u64 {
                    let n = place_plate_seeds(bounds, &params, &mut rng(seed)).len();
                    let target_usize = usize::from(target);
                    let lo = target_usize.saturating_sub(target_usize / 2);
                    let hi = target_usize + target_usize;
                    assert!(
                        (lo..=hi).contains(&n),
                        "target {target}, seed {seed}: got {n} seeds, expected in [{lo}, {hi}]"
                    );
                }
            }
        }
    }

    // ------------- Bounds handling -------------

    mod bounds {
        use super::*;

        #[test]
        fn all_seeds_are_in_bounds() {
            let bounds = MapBounds::new(240, 150);
            let seeds = place_plate_seeds(bounds, &TectonicsParams::defaults(), &mut rng(7));
            for s in seeds {
                assert!(bounds.contains(s), "seed {s:?} out of bounds");
            }
        }

        #[test]
        fn small_map_still_produces_seeds() {
            let bounds = MapBounds::new(40, 30);
            let seeds = place_plate_seeds(bounds, &TectonicsParams::defaults(), &mut rng(0));
            assert!(!seeds.is_empty(), "small map should still produce seeds");
        }
    }
}
