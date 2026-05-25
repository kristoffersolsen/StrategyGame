//! Hill classification pass (step 9).
//!
//! Determines which tiles are "hilly" — a roughness attribute
//! independent of height level that reduces movement speed and provides
//! a combat bonus (design §1.9, step 9).
//!
//! # Algorithm
//!
//! 1. Run a multi-source BFS from all boundary tiles to compute the
//!    hex-distance from every tile to its nearest tectonic boundary.
//! 2. For each tile, linearly interpolate the hilliness noise threshold
//!    between `hill_near_boundary_threshold` (at distance 0) and
//!    `hill_interior_threshold` (at distance ≥ `hill_influence_radius`).
//! 3. Evaluate a wrap-aware `OpenSimplex` noise at each tile; the tile
//!    is hilly if the noise value exceeds the interpolated threshold.
//!
//! The noise threshold approach produces spatially coherent hill patches
//! rather than independent per-tile random draws.  Tectonic boundaries
//! are reliably rougher than plate interiors due to the lower threshold.
//!
//! # Tuning
//!
//! The preliminary parameter set (`hill_near_boundary_threshold`,
//! `hill_interior_threshold`, `hill_influence_radius`,
//! `hill_noise_frequency`) is expected to be refined once the full
//! pipeline produces visible maps.

use std::collections::VecDeque;
use std::f64::consts::TAU;

use coords::{Coord, MapBounds};
use noise::{NoiseFn, OpenSimplex};

use crate::Field;
use crate::params::TectonicsParams;
use crate::world_data::BoundaryType;

// ------------- Torus-wrapping constants (same convention as perturbation.rs) -------------

const TORUS_RADIUS: f64 = 1.0 / (2.0 * std::f64::consts::PI);
const NOISE_Z_SCALE: f64 = 1.0 / 150.0;

// ------------- Sentinel for unvisited BFS tiles -------------

/// Sentinel value meaning "no boundary within max search range."
const UNVISITED: u32 = u32::MAX;

// ------------- Public entry point -------------

/// Classifies each tile as hilly or flat based on proximity to tectonic
/// boundaries and a wrap-aware noise field.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `boundary_type` — per-tile boundary classification from step 5.
/// * `params` — tectonics tuning parameters.
/// * `noise_seed` — `OpenSimplex` seed for the hilliness noise field.
pub(crate) fn classify_hills(
    bounds: MapBounds,
    boundary_type: &Field<Option<BoundaryType>>,
    params: &TectonicsParams,
    noise_seed: u32,
) -> Field<bool> {
    let dist_to_boundary = boundary_distance_field(bounds, boundary_type, params);
    let noise = OpenSimplex::new(noise_seed);
    let width_f64 = f64::from(bounds.width());
    let freq = f64::from(params.hill_noise_frequency);

    Field::from_fn(bounds, |tile| {
        let dist = dist_to_boundary[tile];
        let threshold = interpolate_threshold(dist, params);
        let nx = noise_input(tile, width_f64, freq);
        #[allow(clippy::cast_possible_truncation)]
        let value = noise.get(nx) as f32;
        value > threshold
    })
}

// ------------- BFS distance field -------------

/// Computes the hex-distance from every tile to the nearest boundary
/// tile via multi-source BFS.
///
/// Tiles with no boundary within `hill_influence_radius` are assigned
/// distance = `hill_influence_radius` (capped, so the interpolation
/// gives them exactly the interior threshold).
fn boundary_distance_field(
    bounds: MapBounds,
    boundary_type: &Field<Option<BoundaryType>>,
    params: &TectonicsParams,
) -> Field<u32> {
    let max_dist = params.hill_influence_radius;
    let mut distance = Field::filled(bounds, UNVISITED);
    let mut queue: VecDeque<Coord> = VecDeque::new();

    // Seed BFS from all boundary tiles (iterated in row-major order for determinism).
    for (coord, &bt) in boundary_type.iter_coords() {
        if bt.is_some() {
            distance.set(coord, 0);
            queue.push_back(coord);
        }
    }

    // BFS expansion up to max_dist.
    while let Some(current) = queue.pop_front() {
        let current_dist = distance[current];
        if current_dist >= max_dist {
            continue;
        }

        for neighbor in bounds.neighbors(current).iter().flatten() {
            if distance[*neighbor] == UNVISITED {
                distance.set(*neighbor, current_dist + 1);
                queue.push_back(*neighbor);
            }
        }
    }

    // Cap unvisited tiles at max_dist so they get the interior threshold.
    for cell in &mut distance {
        if *cell == UNVISITED {
            *cell = max_dist;
        }
    }

    distance
}

// ------------- Threshold interpolation -------------

/// Returns the hilliness noise threshold for a tile at `dist` tiles
/// from the nearest boundary.
///
/// Linearly interpolates from `hill_near_boundary_threshold` at
/// `dist = 0` to `hill_interior_threshold` at
/// `dist >= hill_influence_radius`.
fn interpolate_threshold(dist: u32, params: &TectonicsParams) -> f32 {
    if params.hill_influence_radius == 0 {
        return params.hill_interior_threshold;
    }

    #[allow(clippy::cast_precision_loss)]
    let t = (dist as f32 / params.hill_influence_radius as f32).clamp(0.0, 1.0);
    params.hill_near_boundary_threshold * (1.0 - t) + params.hill_interior_threshold * t
}

// ------------- Noise utility -------------

fn noise_input(tile: Coord, width_f64: f64, frequency: f64) -> [f64; 3] {
    let q_f64 = f64::from(tile.q);
    let r_f64 = f64::from(tile.r);
    let theta = TAU * q_f64 / width_f64;
    [
        theta.cos() * TORUS_RADIUS * frequency,
        theta.sin() * TORUS_RADIUS * frequency,
        r_f64 * NOISE_Z_SCALE * frequency,
    ]
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::TectonicsParams;

    fn default_params() -> TectonicsParams {
        TectonicsParams::defaults()
    }

    fn empty_boundary_field(bounds: MapBounds) -> Field<Option<BoundaryType>> {
        Field::filled(bounds, None)
    }

    fn single_boundary_field(bounds: MapBounds, coord: Coord) -> Field<Option<BoundaryType>> {
        let mut field = Field::filled(bounds, None);
        field.set(coord, Some(BoundaryType::Convergent));
        field
    }

    // ------------- BFS distance field -------------

    mod bfs {
        use super::*;

        #[test]
        fn boundary_tile_has_distance_zero() {
            let bounds = MapBounds::new(20, 15);
            let boundary_coord = Coord::new(10, 7);
            let boundary = single_boundary_field(bounds, boundary_coord);
            let params = default_params();
            let dist = boundary_distance_field(bounds, &boundary, &params);
            assert_eq!(dist[boundary_coord], 0);
        }

        #[test]
        fn adjacent_tile_has_distance_one() {
            let bounds = MapBounds::new(20, 15);
            let boundary_coord = Coord::new(10, 7);
            let boundary = single_boundary_field(bounds, boundary_coord);
            let params = default_params();
            let dist = boundary_distance_field(bounds, &boundary, &params);

            // All valid neighbors of the boundary tile must have distance 1.
            for neighbor in bounds.neighbors(boundary_coord).iter().flatten() {
                assert_eq!(
                    dist[*neighbor], 1,
                    "neighbor {neighbor:?} should have distance 1",
                );
            }
        }

        #[test]
        fn no_boundary_tiles_gives_max_dist_everywhere() {
            let bounds = MapBounds::new(10, 8);
            let boundary = empty_boundary_field(bounds);
            let params = default_params();
            let dist = boundary_distance_field(bounds, &boundary, &params);

            for &d in &dist {
                assert_eq!(
                    d, params.hill_influence_radius,
                    "all tiles should be capped at max_dist"
                );
            }
        }

        #[test]
        fn distance_is_bounded_by_max_radius() {
            let bounds = MapBounds::new(30, 20);
            // Single boundary tile at the corner; distant tiles will exceed the radius.
            let boundary = single_boundary_field(bounds, Coord::new(0, 0));
            let params = default_params();
            let dist = boundary_distance_field(bounds, &boundary, &params);

            let max = params.hill_influence_radius;
            for &d in &dist {
                assert!(d <= max, "distance {d} exceeds max_dist {max}");
            }
        }
    }

    // ------------- Threshold interpolation -------------

    mod threshold {
        use super::*;

        #[allow(clippy::float_cmp)]
        #[test]
        fn at_distance_zero_returns_near_boundary_threshold() {
            let params = default_params();
            let t = interpolate_threshold(0, &params);
            assert_eq!(t, params.hill_near_boundary_threshold);
        }

        #[allow(clippy::float_cmp)]
        #[test]
        fn at_max_distance_returns_interior_threshold() {
            let params = default_params();
            let t = interpolate_threshold(params.hill_influence_radius, &params);
            assert_eq!(t, params.hill_interior_threshold);
        }

        #[test]
        fn threshold_is_monotone_with_distance() {
            let params = default_params();
            let mut prev = interpolate_threshold(0, &params);
            for d in 1..=params.hill_influence_radius {
                let curr = interpolate_threshold(d, &params);
                // Near-boundary threshold < interior threshold, so threshold
                // increases with distance.
                assert!(
                    curr >= prev,
                    "threshold should be non-decreasing with distance"
                );
                prev = curr;
            }
        }
    }

    // ------------- Hill classification -------------

    mod classification {
        use super::*;

        #[test]
        fn output_dimensions_match_bounds() {
            let bounds = MapBounds::new(30, 20);
            let boundary = empty_boundary_field(bounds);
            let params = default_params();
            let hills = classify_hills(bounds, &boundary, &params, 0);
            assert_eq!(hills.len(), 30 * 20);
        }

        #[test]
        fn hills_near_boundary_are_more_frequent_than_interior() {
            let bounds = MapBounds::new(60, 40);
            // Place a boundary line at r=20 across the entire width.
            let mut boundary = Field::filled(bounds, None);
            let w = bounds.width().cast_signed();
            for q in 0..w {
                boundary.set(Coord::new(q, 20), Some(BoundaryType::Convergent));
            }

            let params = default_params();
            let hills = classify_hills(bounds, &boundary, &params, 42);

            // Count hilly tiles in the two rows adjacent to the boundary vs far away.
            let near: usize = [-1, 0, 1]
                .iter()
                .map(|dr| {
                    let r = 20 + dr;
                    (0..w).filter(|&q| hills[Coord::new(q, r)]).count()
                })
                .sum();
            let far: usize = [5, 10, 15]
                .iter()
                .map(|&r| (0..w).filter(|&q| hills[Coord::new(q, r)]).count())
                .sum();

            #[allow(clippy::cast_precision_loss)]
            let near_fraction = near as f32 / (3.0 * w as f32);
            #[allow(clippy::cast_precision_loss)]
            let far_fraction = far as f32 / (3.0 * w as f32);

            assert!(
                near_fraction > far_fraction,
                "hills near boundary ({near_fraction:.2}) should be more frequent \
                 than interior ({far_fraction:.2})"
            );
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_output() {
            let bounds = MapBounds::new(30, 20);
            let boundary = single_boundary_field(bounds, Coord::new(15, 10));
            let params = default_params();

            let a = classify_hills(bounds, &boundary, &params, 7);
            let b = classify_hills(bounds, &boundary, &params, 7);
            assert_eq!(a.as_slice(), b.as_slice());
        }

        #[test]
        fn different_seeds_produce_different_output() {
            let bounds = MapBounds::new(30, 20);
            let boundary = single_boundary_field(bounds, Coord::new(15, 10));
            let params = default_params();

            let a = classify_hills(bounds, &boundary, &params, 1);
            let b = classify_hills(bounds, &boundary, &params, 2);
            // Vanishingly unlikely for all tiles to match with different seeds.
            let differ = a.as_slice().iter().zip(b.as_slice()).any(|(x, y)| x != y);
            assert!(
                differ,
                "different seeds should produce different hill patterns"
            );
        }
    }
}
