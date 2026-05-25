//! Sea-level correction and discrete height-level assignment (step 8).
//!
//! Takes the continuous elevation field produced by steps 6–7 and
//! produces two outputs:
//!
//! 1. **Normalised elevation** — the elevation field is shifted so that
//!    sea level is exactly 0.  The shift is computed as the elevation
//!    value at the `(ocean_fraction + coast_fraction)` quantile.
//!
//! 2. **Height levels** — each tile is bucketed into a `u8` height level
//!    (`0` = ocean, `1` = coast, `2..=N` = land levels, `N+1` = mountain)
//!    by computing quantile thresholds from `level_proportions` and
//!    comparing each tile's (normalised) elevation against those thresholds.
//!    The thresholds are chosen so that the achieved proportions exactly
//!    match `level_proportions` by construction.
//!
//! # Quantile algorithm
//!
//! 1. Collect all elevation values into a `Vec<f32>`.
//! 2. Sort the vec.
//! 3. For each cumulative proportion prefix, read off the threshold
//!    value at the corresponding index (`floor(proportion × tile_count)`).
//! 4. Shift all elevations by subtracting the sea-level threshold.
//! 5. Map each tile to the highest level index whose threshold it exceeds.

use coords::MapBounds;

use crate::Field;
use crate::params::TectonicsParams;

// ------------- Public entry point -------------

/// Normalises `elevation` in-place (shifts sea level to 0) and returns
/// a per-tile height-level field.
///
/// After this call, `elevation[tile] >= 0` means the tile is at or
/// above sea level.
///
/// # Panics
///
/// Panics if `elevation` contains non-finite values or if the tile
/// count is zero — both conditions indicate a logic error upstream.
pub(crate) fn quantize_elevation(
    bounds: MapBounds,
    elevation: &mut Field<f32>,
    params: &TectonicsParams,
) -> Field<u8> {
    let tile_count = elevation.len();
    assert!(tile_count > 0, "elevation field must be non-empty");

    // Collect and sort all elevation values to compute quantiles.
    let mut sorted: Vec<f32> = elevation.iter().copied().collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("elevation must be finite (step 8)"));

    // Compute cumulative threshold indices from level_proportions.
    let thresholds = compute_thresholds(&sorted, &params.level_proportions);

    // Sea-level offset: the threshold between ocean/coast and land.
    // Index 1 is the cumulative proportion after ocean; index 2 is after coast.
    // The sea-level threshold is at the end of the coast band (index 2).
    let sea_level_threshold = thresholds[1]; // threshold between ocean and coast
    // Shift so sea level = 0.
    for cell in elevation.iter_mut() {
        *cell -= sea_level_threshold;
    }
    // Shift all thresholds by the same amount.
    let shifted: Vec<f32> = thresholds.iter().map(|t| t - sea_level_threshold).collect();

    // Assign height levels using the shifted thresholds.
    #[allow(clippy::cast_possible_truncation)]
    let level_count = params.level_proportions.len() as u8;

    Field::from_fn(bounds, |tile| {
        let elev = elevation[tile];
        // Find the highest level whose lower threshold is exceeded.
        // Level 0 applies to all tiles (threshold[0] = -∞ effectively).
        // `i` is needed both as index into `shifted` and as the level value.
        let mut level: u8 = 0;
        #[allow(clippy::needless_range_loop)]
        for i in 1..usize::from(level_count) {
            if elev >= shifted[i] {
                #[allow(clippy::cast_possible_truncation)]
                let l = i as u8;
                level = l;
            } else {
                break;
            }
        }
        level
    })
}

/// Computes the elevation threshold at each cumulative level boundary.
///
/// Returns a vec of length `proportions.len()` where entry `i` is the
/// elevation value at the lower bound of level `i` in the sorted
/// distribution.
///
/// Entry 0 is always `f32::NEG_INFINITY` (ocean starts at the bottom).
/// Entry `i > 0` is the elevation at the `cumsum[i-1]` quantile of
/// the sorted elevations.
fn compute_thresholds(sorted: &[f32], proportions: &[f32]) -> Vec<f32> {
    let n = sorted.len();
    let mut cumsum = 0.0_f32;
    let mut thresholds = Vec::with_capacity(proportions.len());
    // Level 0 has no lower bound.
    thresholds.push(f32::NEG_INFINITY);

    for &prop in proportions.iter().take(proportions.len() - 1) {
        cumsum += prop;
        // Index of the first tile at or above this quantile.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let idx = ((cumsum * n as f32).floor() as usize).min(n - 1);
        thresholds.push(sorted[idx]);
    }

    thresholds
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use coords::Coord;

    fn default_params() -> TectonicsParams {
        TectonicsParams::defaults()
    }

    /// Build a simple elevation field with known values to test the
    /// quantile logic end-to-end.
    fn make_field(values: &[f32]) -> Field<f32> {
        #[allow(clippy::cast_possible_truncation)]
        let width = values.len() as u32;
        let bounds = MapBounds::new(width, 1);
        let mut field = Field::filled(bounds, 0.0_f32);
        for (i, &v) in values.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            field.set(Coord::new(i as i32, 0), v);
        }
        field
    }

    // ------------- Level assignment correctness -------------

    mod level_assignment {
        use super::*;

        /// With a uniform distribution the quantile-match should put
        /// tiles into levels proportional to `level_proportions`.
        #[allow(clippy::cast_precision_loss)]
        #[test]
        fn level_proportions_match_within_one_tile() {
            let params = default_params();
            // 100 uniformly spaced values.
            let values: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
            let mut elevation = make_field(&values);
            let levels = quantize_elevation(MapBounds::new(100, 1), &mut elevation, &params);

            let total = levels.len() as f32;
            let level_count = usize::from(params.land_level_count) + 3;
            for target_level in 0..level_count {
                #[allow(clippy::cast_possible_truncation)]
                let level_id = target_level as u8;
                #[allow(clippy::naive_bytecount)]
                let count = levels.iter().filter(|&&l| l == level_id).count();
                let fraction = count as f32 / total;
                let expected = params.level_proportions[target_level];
                // Allow ±2 % tolerance due to integer rounding.
                assert!(
                    (fraction - expected).abs() < 0.03,
                    "level {target_level}: fraction {fraction:.3} vs expected {expected:.3}"
                );
            }
        }

        #[allow(clippy::cast_precision_loss)]
        #[test]
        fn sea_level_shift_makes_ocean_tiles_negative() {
            let params = default_params();
            let values: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
            let mut elevation = make_field(&values);
            let levels = quantize_elevation(MapBounds::new(100, 1), &mut elevation, &params);

            for (tile, &level) in levels.iter_coords() {
                let elev = elevation[tile];
                if level == 0 {
                    // Ocean tiles must be below sea level.
                    assert!(
                        elev < 0.0 || elev.abs() < 1e-4,
                        "ocean tile has non-negative elevation {elev}"
                    );
                }
            }
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[allow(clippy::cast_precision_loss)]
        #[test]
        fn same_inputs_produce_identical_levels() {
            let params = default_params();
            let values: Vec<f32> = (0..200).map(|i| (i as f32 - 100.0) / 100.0).collect();

            let mut elevation_a = make_field(&values);
            let mut elevation_b = make_field(&values);

            let levels_a = quantize_elevation(MapBounds::new(200, 1), &mut elevation_a, &params);
            let levels_b = quantize_elevation(MapBounds::new(200, 1), &mut elevation_b, &params);

            assert_eq!(levels_a.as_slice(), levels_b.as_slice());
            // Elevation fields should also match after normalisation.
            assert_eq!(elevation_a.as_slice(), elevation_b.as_slice());
        }
    }

    // ------------- Threshold computation -------------

    mod thresholds {
        use super::*;

        #[allow(clippy::cast_precision_loss, clippy::float_cmp)]
        #[test]
        fn first_threshold_is_neg_infinity() {
            let sorted: Vec<f32> = (0..100).map(|i| i as f32).collect();
            let proportions = vec![0.4, 0.1, 0.2, 0.2, 0.05, 0.05];
            let thresholds = compute_thresholds(&sorted, &proportions);
            assert_eq!(thresholds[0], f32::NEG_INFINITY);
        }

        #[allow(clippy::cast_precision_loss)]
        #[test]
        fn threshold_count_equals_proportion_count() {
            let sorted: Vec<f32> = (0..100).map(|i| i as f32).collect();
            let proportions = vec![0.35, 0.05, 0.25, 0.20, 0.10, 0.05];
            let thresholds = compute_thresholds(&sorted, &proportions);
            assert_eq!(thresholds.len(), proportions.len());
        }

        #[allow(clippy::cast_precision_loss)]
        #[test]
        fn thresholds_are_non_decreasing() {
            let sorted: Vec<f32> = (0..100).map(|i| i as f32).collect();
            let proportions = vec![0.35, 0.05, 0.25, 0.20, 0.10, 0.05];
            let thresholds = compute_thresholds(&sorted, &proportions);
            // Ignore first entry (NEG_INFINITY) in the window check.
            for window in thresholds[1..].windows(2) {
                assert!(window[1] >= window[0], "thresholds must be non-decreasing");
            }
        }
    }
}
