//! Plate property assignment (step 4 of the tectonics stage).
//!
//! Assigns three properties to each plate (design §1.9 step 4):
//!
//! 1. **Type** — continental or oceanic, sampled probabilistically
//!    from the `land_fraction` parameter (ADR-0002).
//! 2. **Motion vector** — a random 2D drift direction and magnitude,
//!    consumed by boundary classification (step 5).
//! 3. **Mineral character** — a weighted random geological category,
//!    consumed by resource placement (design §1.13).
//!
//! # Motion vector generation
//!
//! The direction is drawn uniformly from `[0, 2π)`. The magnitude is
//! drawn uniformly from
//! `[motion_magnitude_min, motion_magnitude_max]`. The scale is
//! dimensionless — only the *relative* motion between adjacent plates
//! matters for boundary classification, not the absolute values.
//!
//! # Mineral character sampling
//!
//! Weighted random selection over [`MineralCharacter::ALL`] using the
//! `mineral_weights` parameter. Weights are normalised to a cumulative
//! distribution and sampled via a single `f32` draw, avoiding any
//! heap allocation per plate.

use coords::{Direction, MapBounds};
use rand::RngExt;
use std::f32::consts::TAU;

use crate::Field;
use crate::params::TectonicsParams;
use crate::rng::StageRng;
use crate::world_data::{MineralCharacter, MotionVector, Plate, PlateId, PlateType};

// ------------- Algorithm constants -------------

/// Number of local-majority smoothing passes used for plate-type clumping.
const CLUMPING_PASS_COUNT: u8 = 3;

/// Half-neighbour direction set used to count plate adjacencies once.
const ADJACENCY_DIRECTIONS: [Direction; 3] =
    [Direction::East, Direction::SouthEast, Direction::SouthWest];

/// Assigns type, motion vector, and mineral character to each plate.
///
/// Returns a `Vec<Plate>` parallel to `seeds` — entry `i` describes
/// the plate whose seed is at `seeds[i]`. The length of the returned
/// vec always equals `plate_count`.
///
/// # Panics
///
/// Panics if `params.mineral_weights` sums to zero or has a different
/// length than [`MineralCharacter::ALL`]. Both conditions are rejected
/// by [`TectonicsParams::validate`] before this function is reached.
pub(crate) fn assign_plate_properties(
    plate_count: usize,
    params: &TectonicsParams,
    rng: &mut StageRng,
) -> Vec<Plate> {
    // Pre-compute the cumulative mineral weight distribution once,
    // outside the per-plate loop.
    let mineral_cdf = build_mineral_cdf(&params.mineral_weights);

    (0..plate_count)
        .map(|_| Plate {
            plate_type: sample_plate_type(params, rng),
            motion: sample_motion(params, rng),
            mineral_character: sample_mineral(&mineral_cdf, rng),
        })
        .collect()
}

/// Clumps oceanic and continental plate types across the plate adjacency graph.
///
/// The initial type assignment is independent per plate. This pass
/// optionally smooths those assignments by letting a plate adopt the
/// majority type of its adjacent plates. `clumping_index = 0.0` is a
/// strict no-op and consumes no RNG, preserving the original output.
pub(crate) fn clump_plate_types(
    bounds: MapBounds,
    plate_id: &Field<PlateId>,
    plates: &mut [Plate],
    params: &TectonicsParams,
    rng: &mut StageRng,
) {
    if params.clumping_index <= 0.0 || plates.len() <= 1 {
        return;
    }

    for _ in 0..CLUMPING_PASS_COUNT {
        let neighbor_counts = plate_type_neighbor_counts(bounds, plate_id, plates);
        let mut next_types: Vec<PlateType> = plates.iter().map(|plate| plate.plate_type).collect();

        for (plate_index, counts) in neighbor_counts.iter().enumerate() {
            let current = plates[plate_index].plate_type;
            let Some(majority) = majority_type(*counts) else {
                continue;
            };
            if majority == current {
                continue;
            }
            if rng.random_bool(f64::from(params.clumping_index)) {
                next_types[plate_index] = majority;
            }
        }

        for (plate, next_type) in plates.iter_mut().zip(next_types) {
            plate.plate_type = next_type;
        }
    }
}

fn plate_type_neighbor_counts(
    bounds: MapBounds,
    plate_id: &Field<PlateId>,
    plates: &[Plate],
) -> Vec<[u32; 2]> {
    let mut counts = vec![[0_u32; 2]; plates.len()];

    for (coord, &own_id) in plate_id.iter_coords() {
        for direction in ADJACENCY_DIRECTIONS {
            let Some(neighbor) = bounds.neighbor(coord, direction) else {
                continue;
            };
            let other_id = plate_id[neighbor];
            if own_id == other_id {
                continue;
            }

            let own_index = usize::from(own_id);
            let other_index = usize::from(other_id);
            counts[own_index][plate_type_index(plates[other_index].plate_type)] += 1;
            counts[other_index][plate_type_index(plates[own_index].plate_type)] += 1;
        }
    }

    counts
}

fn majority_type(counts: [u32; 2]) -> Option<PlateType> {
    match counts {
        [continental, oceanic] if continental > oceanic => Some(PlateType::Continental),
        [continental, oceanic] if oceanic > continental => Some(PlateType::Oceanic),
        _ => None,
    }
}

fn plate_type_index(plate_type: PlateType) -> usize {
    match plate_type {
        PlateType::Continental => 0,
        PlateType::Oceanic => 1,
    }
}

// ------------- Plate type -------------

/// Samples a plate type probabilistically from `land_fraction`
/// (ADR-0002).
///
/// Continental with probability `land_fraction`; oceanic otherwise.
/// The exact land fraction in the final map is enforced by quantile
/// thresholding at step 9, not by this assignment.
fn sample_plate_type(params: &TectonicsParams, rng: &mut StageRng) -> PlateType {
    // land_fraction lives on AdvancedParams, not TectonicsParams.
    // It is passed through the generation context; for now the
    // threshold is read from the first level_proportions entry which
    // encodes the ocean fraction. Land fraction = 1 - ocean fraction.
    //
    // level_proportions[0] is ocean, [1] is coast; both are below sea
    // level. The continental probability is 1 minus their sum.
    let below_sea_level: f32 = params
        .level_proportions
        .iter()
        .take(2) // ocean + coast
        .sum();
    let continental_probability = f64::from(1.0 - below_sea_level);
    if rng.random_bool(continental_probability.clamp(0.0, 1.0)) {
        PlateType::Continental
    } else {
        PlateType::Oceanic
    }
}

// ------------- Motion vector -------------

/// Samples a random motion vector with angle uniform in `[0, 2π)` and
/// magnitude uniform in `[motion_magnitude_min, motion_magnitude_max]`.
fn sample_motion(params: &TectonicsParams, rng: &mut StageRng) -> MotionVector {
    let angle: f32 = rng.random_range(0.0..TAU);
    let magnitude: f32 =
        rng.random_range(params.motion_magnitude_min..=params.motion_magnitude_max);
    let (sin, cos) = angle.sin_cos();
    MotionVector {
        dx: cos * magnitude,
        dy: sin * magnitude,
    }
}

// ------------- Mineral character -------------

/// Builds a normalised cumulative distribution function over
/// [`MineralCharacter::ALL`] from the given weights.
///
/// The returned vec has the same length as `weights`. Entry `i` is
/// the cumulative probability of selecting variant `i` or any earlier
/// variant.
///
/// # Panics
///
/// Panics if `weights` is empty or sums to zero.
fn build_mineral_cdf(weights: &[f32]) -> Vec<f32> {
    let total: f32 = weights.iter().sum();
    assert!(
        total > 0.0,
        "mineral_weights must sum to > 0; \
         TectonicsParams::validate rejects this earlier"
    );
    let mut cdf = Vec::with_capacity(weights.len());
    let mut cumulative = 0.0_f32;
    for &w in weights {
        cumulative += w / total;
        cdf.push(cumulative);
    }
    // Force the last entry to exactly 1.0 to avoid floating-point
    // drift causing the final variant to never be selected.
    if let Some(last) = cdf.last_mut() {
        *last = 1.0;
    }
    cdf
}

/// Samples a [`MineralCharacter`] variant from the pre-built CDF.
fn sample_mineral(cdf: &[f32], rng: &mut StageRng) -> MineralCharacter {
    let roll: f32 = rng.random_range(0.0..1.0_f32);
    let idx = cdf
        .iter()
        .position(|&threshold| roll < threshold)
        .unwrap_or(cdf.len() - 1); // saturate to last if roll == 1.0
    MineralCharacter::ALL[idx]
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{MasterRng, StageId};

    fn rng(seed: u64) -> StageRng {
        MasterRng::new(seed).fork(StageId::Tectonics)
    }

    fn default_params() -> TectonicsParams {
        TectonicsParams::defaults()
    }

    fn plate(plate_type: PlateType) -> Plate {
        Plate {
            plate_type,
            motion: MotionVector { dx: 0.0, dy: 0.0 },
            mineral_character: MineralCharacter::Ferrous,
        }
    }

    // ------------- Basic properties -------------

    mod basic {
        use super::*;

        #[test]
        fn returns_correct_count() {
            let plates = assign_plate_properties(10, &default_params(), &mut rng(0));
            assert_eq!(plates.len(), 10);
        }

        #[test]
        fn zero_plates_returns_empty_vec() {
            let plates = assign_plate_properties(0, &default_params(), &mut rng(0));
            assert!(plates.is_empty());
        }

        #[test]
        fn motion_magnitudes_within_range() {
            let params = default_params();
            let plates = assign_plate_properties(32, &params, &mut rng(1));
            for plate in &plates {
                let mag =
                    (plate.motion.dx * plate.motion.dx + plate.motion.dy * plate.motion.dy).sqrt();
                assert!(
                    mag >= params.motion_magnitude_min && mag <= params.motion_magnitude_max,
                    "magnitude {mag} outside [{}, {}]",
                    params.motion_magnitude_min,
                    params.motion_magnitude_max
                );
            }
        }

        #[test]
        fn all_mineral_characters_are_valid_variants() {
            let plates = assign_plate_properties(64, &default_params(), &mut rng(2));
            // Compile-time exhaustiveness: matching on MineralCharacter
            // ensures all variants are covered.
            for plate in &plates {
                let _ = match plate.mineral_character {
                    MineralCharacter::Ferrous => 0,
                    MineralCharacter::Precious => 1,
                    MineralCharacter::Gems => 2,
                    MineralCharacter::Hydrocarbons => 3,
                    MineralCharacter::Industrial => 4,
                };
            }
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_output() {
            let params = default_params();
            let a = assign_plate_properties(10, &params, &mut rng(42));
            let b = assign_plate_properties(10, &params, &mut rng(42));
            // Compare field-by-field since Plate doesn't derive PartialEq
            // (MotionVector contains f32).
            assert_eq!(a.len(), b.len());
            for (pa, pb) in a.iter().zip(b.iter()) {
                assert_eq!(pa.plate_type, pb.plate_type);
                assert_eq!(pa.mineral_character, pb.mineral_character);
                // f32 exact equality is valid here: same RNG stream must
                // produce byte-identical floats.
                assert_eq!(pa.motion.dx.to_bits(), pb.motion.dx.to_bits());
                assert_eq!(pa.motion.dy.to_bits(), pb.motion.dy.to_bits());
            }
        }

        #[test]
        fn different_seeds_produce_different_output() {
            let params = default_params();
            let a = assign_plate_properties(10, &params, &mut rng(1));
            let b = assign_plate_properties(10, &params, &mut rng(2));
            // Vanishingly unlikely for all 10 plate types to match.
            let all_same = a
                .iter()
                .zip(b.iter())
                .all(|(pa, pb)| pa.plate_type == pb.plate_type);
            assert!(
                !all_same,
                "different seeds should produce different plate types"
            );
        }
    }

    // ------------- Plate type distribution -------------

    mod plate_type_distribution {
        use super::*;

        /// Over many plates, the continental fraction should be roughly
        /// consistent with the land fraction implied by `level_proportions`.
        /// This is a loose statistical test — it just confirms the
        /// distribution is in the right ballpark, not an exact value.
        #[allow(clippy::cast_precision_loss)] // plate counts are small (<1000), no precision loss
        #[test]
        fn continental_fraction_close_to_land_fraction() {
            let params = default_params();
            let below_sea: f32 = params.level_proportions.iter().take(2).sum();
            let expected_continental = 1.0 - below_sea;

            let plates = assign_plate_properties(200, &params, &mut rng(0));
            let continental_count = plates
                .iter()
                .filter(|p| p.plate_type == PlateType::Continental)
                .count();
            let fraction = continental_count as f32 / plates.len() as f32;

            // Allow generous tolerance: with 200 samples, 3-sigma is ~10%.
            let tolerance = 0.15;
            assert!(
                (fraction - expected_continental).abs() < tolerance,
                "continental fraction {fraction:.2} too far from \
                 expected {expected_continental:.2}"
            );
        }
    }

    // ------------- Mineral CDF -------------

    mod mineral_cdf {
        use super::*;

        #[allow(clippy::float_cmp)] // verifying the explicit `*last = 1.0` assignment
        #[test]
        fn cdf_last_entry_is_exactly_one() {
            let weights = TectonicsParams::defaults().mineral_weights;
            let cdf = build_mineral_cdf(&weights);
            assert_eq!(
                *cdf.last().unwrap(),
                1.0,
                "last CDF entry must be exactly 1.0"
            );
        }

        #[test]
        fn cdf_is_monotonically_non_decreasing() {
            let weights = TectonicsParams::defaults().mineral_weights;
            let cdf = build_mineral_cdf(&weights);
            for window in cdf.windows(2) {
                assert!(window[1] >= window[0], "CDF must be non-decreasing");
            }
        }

        #[allow(clippy::cast_precision_loss)] // n == MineralCharacter::ALL.len() ≤ 256, no precision loss
        #[test]
        fn uniform_weights_produce_equal_probabilities() {
            let n = MineralCharacter::ALL.len();
            let weights = vec![1.0_f32; n];
            let cdf = build_mineral_cdf(&weights);
            let expected_step = 1.0 / n as f32;
            for (i, &entry) in cdf.iter().enumerate() {
                let expected = expected_step * (i + 1) as f32;
                assert!(
                    (entry - expected).abs() < 1e-5,
                    "cdf[{i}] = {entry}, expected ~{expected}"
                );
            }
        }

        /// With a weight of zero on all variants except one, only that
        /// variant should ever be sampled.
        #[test]
        fn zero_weight_variants_are_never_sampled() {
            let mut weights = vec![0.0_f32; MineralCharacter::ALL.len()];
            weights[2] = 1.0; // Only Gems
            let params = TectonicsParams {
                mineral_weights: weights,
                ..TectonicsParams::defaults()
            };
            let plates = assign_plate_properties(50, &params, &mut rng(5));
            for plate in &plates {
                assert_eq!(
                    plate.mineral_character,
                    MineralCharacter::Gems,
                    "only Gems should be sampled when its weight is 1 and others are 0"
                );
            }
        }
    }

    // ------------- Plate type clumping -------------

    mod plate_type_clumping {
        use super::*;
        use coords::MapBounds;

        #[test]
        fn zero_clumping_index_is_noop() {
            let bounds = MapBounds::new(3, 3);
            let plate_id = Field::from_fn(bounds, |coord| {
                PlateId::try_from(coord.q).expect("test coord q fits in PlateId")
            });
            let mut plates = vec![
                plate(PlateType::Continental),
                plate(PlateType::Oceanic),
                plate(PlateType::Continental),
            ];
            let original_types: Vec<PlateType> =
                plates.iter().map(|plate| plate.plate_type).collect();
            let mut rng = rng(42);

            clump_plate_types(bounds, &plate_id, &mut plates, &default_params(), &mut rng);

            let clumped_types: Vec<PlateType> =
                plates.iter().map(|plate| plate.plate_type).collect();
            assert_eq!(clumped_types, original_types);
        }

        #[test]
        fn full_clumping_adopts_neighbor_majority_type() {
            let bounds = MapBounds::new(3, 3);
            let plate_id = Field::from_fn(bounds, |coord| {
                PlateId::try_from(coord.q).expect("test coord q fits in PlateId")
            });
            let mut plates = vec![
                plate(PlateType::Continental),
                plate(PlateType::Oceanic),
                plate(PlateType::Continental),
            ];
            let params = TectonicsParams {
                clumping_index: 1.0,
                ..TectonicsParams::defaults()
            };
            let mut rng = rng(42);

            clump_plate_types(bounds, &plate_id, &mut plates, &params, &mut rng);

            assert_eq!(plates[1].plate_type, PlateType::Continental);
        }
    }
}
