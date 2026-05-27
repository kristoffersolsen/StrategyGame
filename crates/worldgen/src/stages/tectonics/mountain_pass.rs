//! Mountain pass carving pass (step 10).
//!
//! Demotes a noise-selected subset of mountain tiles to high-land,
//! producing traversable gaps — passes — through otherwise impassable
//! mountain ranges.
//!
//! # The problem
//!
//! Convergent plate boundaries produce continuous linear ridges: every
//! tile along the boundary accumulates a Gaussian elevation boost, so
//! the resulting mountain band is unbroken even after the Gaussian sigma
//! has been reduced to a few tiles. No sigma value can break a
//! geometrically continuous boundary line into discrete peaks.
//!
//! # Algorithm
//!
//! After height-level quantization (step 8), iterate over every mountain
//! tile and evaluate a wrap-aware `OpenSimplex` noise sample. If the
//! sample exceeds `mountain_pass_threshold`, the tile is a candidate for
//! demotion.
//!
//! When `mountain_pass_neighbor_aware` is `true`, an additional filter
//! requires that at least `mountain_pass_min_mountain_neighbors` of the
//! tile's six neighbors are also at mountain level. This restricts
//! demotion to interior range tiles, producing gaps that cut *through*
//! a range rather than nibbling its edges.
//!
//! # Effect on `level_proportions`
//!
//! This pass demotes mountain tiles to high-land after the quantile
//! thresholding that enforces `level_proportions`. The achieved mountain
//! fraction will therefore be slightly below the target. Compensate by
//! setting `level_proportions[last]` modestly higher than the desired
//! final mountain fraction — see ADR-0005 for the rationale.
//!
//! # Tuning
//!
//! `mountain_pass_threshold` near `0.0` demotes ~50 % of candidates;
//! near `0.5` demotes ~25 %; above `1.0` disables the pass entirely
//! (no `OpenSimplex` sample exceeds 1.0). `mountain_pass_noise_frequency`
//! controls pass spacing: high values produce many short gaps, low
//! values produce few wide ones.

use std::f64::consts::TAU;

use coords::{Coord, MapBounds};
use noise::{NoiseFn, OpenSimplex};

use crate::Field;
use crate::params::TectonicsParams;

// ------------- Torus-wrapping constants -------------

const TORUS_RADIUS: f64 = 1.0 / (2.0 * std::f64::consts::PI);
const NOISE_Z_SCALE: f64 = 1.0 / 150.0;

// ------------- Public entry point -------------

/// Demotes a noise-selected subset of mountain tiles to high-land,
/// carving traversable passes through mountain ranges.
///
/// Mountain level is `land_level_count + 2` (the last entry in
/// `level_proportions`). Demoted tiles become `land_level_count + 1`
/// (high-land), one level below mountain.
///
/// The returned field is a modified copy of `height_level`; the input
/// field is not mutated.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `height_level` — per-tile discrete height level from step 8.
/// * `params` — tectonics tuning parameters.
/// * `noise_seed` — `OpenSimplex` seed for the pass-carving noise field.
pub(crate) fn apply_mountain_passes(
    bounds: MapBounds,
    height_level: &Field<u8>,
    params: &TectonicsParams,
    noise_seed: u32,
) -> Field<u8> {
    // Threshold above 1.0 means the pass is disabled — no OpenSimplex
    // value exceeds 1.0. Return a clone so callers need not special-case.
    if params.mountain_pass_threshold > 1.0 {
        return height_level.clone();
    }

    let mountain_level = params.land_level_count + 2;
    let high_land_level = params.land_level_count + 1;

    let noise = OpenSimplex::new(noise_seed);
    let width_f64 = f64::from(bounds.width());
    let freq = f64::from(params.mountain_pass_noise_frequency);

    Field::from_fn(bounds, |tile| {
        let level = height_level[tile];
        if level != mountain_level {
            return level;
        }

        if params.mountain_pass_neighbor_aware
            && !has_enough_mountain_neighbors(
                tile,
                bounds,
                height_level,
                mountain_level,
                params.mountain_pass_min_mountain_neighbors,
            )
        {
            // Edge tile: leave it as mountain so passes cut through the
            // range interior rather than nibbling its perimeter.
            return level;
        }

        let nx = noise_input(tile, width_f64, freq);
        #[allow(clippy::cast_possible_truncation)]
        let value = noise.get(nx) as f32;

        if value > params.mountain_pass_threshold {
            high_land_level
        } else {
            mountain_level
        }
    })
}

// ------------- Neighbor filter -------------

/// Returns `true` if `tile` has at least `min_mountain_neighbors` of its
/// six hex-neighbors at `mountain_level`.
///
/// Used to restrict pass carving to range interiors. Edge tiles — those
/// already adjacent to high-land or other non-mountain tiles — are left
/// alone so passes punch through ranges rather than eroding their borders.
fn has_enough_mountain_neighbors(
    tile: Coord,
    bounds: MapBounds,
    height_level: &Field<u8>,
    mountain_level: u8,
    min_mountain_neighbors: u8,
) -> bool {
    let count = bounds
        .neighbors(tile)
        .iter()
        .flatten()
        .filter(|&&neighbor| height_level[neighbor] == mountain_level)
        .count();
    #[allow(clippy::cast_possible_truncation)]
    let min = min_mountain_neighbors as usize;
    count >= min
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

    /// Builds a small `height_level` field with the specified mountain
    /// pattern for testing. The field is 20×15.
    fn make_field(bounds: MapBounds, mountain_coords: &[Coord], mountain_level: u8) -> Field<u8> {
        let land_level = mountain_level.saturating_sub(1);
        let mut field = Field::filled(bounds, land_level);
        for &c in mountain_coords {
            field.set(c, mountain_level);
        }
        field
    }

    // ------------- Threshold disables -------------

    mod disabled {
        use super::*;

        /// threshold > 1.0 must leave every tile unchanged.
        #[test]
        fn threshold_above_one_is_noop() {
            let bounds = MapBounds::new(20, 15);
            let mut params = default_params();
            params.mountain_pass_threshold = 1.1;

            let mountain_level = params.land_level_count + 2;
            let coords: Vec<Coord> = (5..=15).map(|q| Coord::new(q, 7)).collect();
            let field = make_field(bounds, &coords, mountain_level);

            let result = apply_mountain_passes(bounds, &field, &params, 42);
            assert_eq!(
                field.as_slice(),
                result.as_slice(),
                "threshold > 1.0 must leave the field unchanged"
            );
        }
    }

    // ------------- Non-mountain tiles preserved -------------

    mod non_mountain_preserved {
        use super::*;

        /// Tiles below mountain level must never be demoted further.
        #[test]
        fn non_mountain_tiles_are_unchanged() {
            let bounds = MapBounds::new(20, 15);
            let mut params = default_params();
            params.mountain_pass_threshold = -1.0; // demote everything that qualifies
            params.mountain_pass_neighbor_aware = false;

            let mountain_level = params.land_level_count + 2;
            // Only one mountain tile; rest are land.
            let field = make_field(bounds, &[Coord::new(10, 7)], mountain_level);

            let result = apply_mountain_passes(bounds, &field, &params, 42);

            for (coord, &original) in field.iter_coords() {
                if original != mountain_level {
                    assert_eq!(
                        result[coord], original,
                        "non-mountain tile at {coord:?} must not change"
                    );
                }
            }
        }
    }

    // ------------- Neighbor-aware filter -------------

    mod neighbor_aware {
        use super::*;

        /// An isolated mountain tile (no mountain neighbors) must not be
        /// demoted when `neighbor_aware` is true, regardless of threshold.
        #[test]
        fn isolated_mountain_not_demoted_when_neighbor_aware() {
            let bounds = MapBounds::new(20, 15);
            let mut params = default_params();
            params.mountain_pass_threshold = -1.0; // demote everything that passes filter
            params.mountain_pass_neighbor_aware = true;
            params.mountain_pass_min_mountain_neighbors = 3;

            let mountain_level = params.land_level_count + 2;
            // Single isolated mountain tile.
            let field = make_field(bounds, &[Coord::new(10, 7)], mountain_level);

            let result = apply_mountain_passes(bounds, &field, &params, 42);

            assert_eq!(
                result[Coord::new(10, 7)],
                mountain_level,
                "isolated mountain tile must not be demoted when neighbor_aware is true"
            );
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_output() {
            let bounds = MapBounds::new(20, 15);
            let params = default_params();
            let mountain_level = params.land_level_count + 2;
            let coords: Vec<Coord> = (3..=17).map(|q| Coord::new(q, 7)).collect();
            let field = make_field(bounds, &coords, mountain_level);

            let a = apply_mountain_passes(bounds, &field, &params, 77);
            let b = apply_mountain_passes(bounds, &field, &params, 77);
            assert_eq!(a.as_slice(), b.as_slice());
        }
    }
}
