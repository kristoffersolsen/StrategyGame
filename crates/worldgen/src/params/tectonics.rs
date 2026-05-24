//! Tuning parameters for the tectonic-plate generation stage.
//!
//! Owns every tunable knob consumed by the tectonics pipeline (design
//! §1.9). Stage 3.1 lands in nine implementation steps; the parameter
//! set is declared up front so the layout is stable across the steps,
//! even though the early steps only consume a few of the fields.
//!
//! See `IMPLEMENTATION_PLAN.md` §3.1 for the step-by-step rollout.

use crate::error::GenerationError;

// ------------- Hard limits -------------

/// Lower bound on [`TectonicsParams::target_plate_count`].
///
/// Below four plates, the map tends toward a single dominant landmass
/// regardless of land-fraction targeting, because each plate covers
/// roughly a quarter of the world and the Voronoi partition cannot
/// produce continental separation.
pub const MIN_TARGET_PLATE_COUNT: u8 = 4;

/// Upper bound on [`TectonicsParams::target_plate_count`].
///
/// Above 32 plates, individual plates become small enough that
/// boundary effects dominate over plate interiors, producing a
/// uniformly mountainous map with no flat regions to settle on.
pub const MAX_TARGET_PLATE_COUNT: u8 = 32;

/// Lower bound on [`TectonicsParams::land_level_count`].
///
/// At least two land levels are required so that "lowland" and
/// "highland" are distinguishable — a single-level world collapses
/// the entire land surface to one plateau.
pub const MIN_LAND_LEVEL_COUNT: u8 = 2;

/// Upper bound on [`TectonicsParams::land_level_count`].
///
/// Discrete levels are encoded as a `u8` height-level value, and
/// the fixed ocean / coast / mountain levels consume three of those.
/// Capping the land range at 16 leaves comfortable headroom inside
/// `u8::MAX` and avoids over-discretising the elevation field.
pub const MAX_LAND_LEVEL_COUNT: u8 = 16;

// ------------- Default values -------------

/// Default value for [`TectonicsParams::target_plate_count`].
///
/// The design document calls for 8–12 plates as the geologically
/// plausible range; ten is the centre of that range and the
/// baseline starting point for tuning.
const DEFAULT_TARGET_PLATE_COUNT: u8 = 10;

/// Default value for [`TectonicsParams::land_level_count`].
///
/// Three land levels (low / mid / high) combined with the fixed
/// ocean, coast, and mountain levels produces a six-band height
/// classification, matching the design-document working assumption.
const DEFAULT_LAND_LEVEL_COUNT: u8 = 3;

/// Default proportions for the six-band height classification at
/// [`DEFAULT_LAND_LEVEL_COUNT`].
///
/// Order matches the [`TectonicsParams::level_proportions`] contract:
/// ocean, coast, each land level low-to-high, mountain. Values are
/// the design-document working baseline before any generation-driven
/// tuning. They sum to `1.0` exactly.
const DEFAULT_LEVEL_PROPORTIONS: [f32; 6] = [0.35, 0.05, 0.25, 0.20, 0.10, 0.05];

/// Tolerance applied to the [`TectonicsParams::level_proportions`]
/// sum check.
///
/// `f32` arithmetic on the proportions can drift by a few ULPs from
/// an exact `1.0` even when the source values were chosen to sum
/// exactly. This tolerance accepts that drift while still rejecting
/// genuinely malformed inputs.
const LEVEL_PROPORTIONS_SUM_TOLERANCE: f32 = 1e-4;

/// Default value for
/// [`TectonicsParams::boundary_perturbation_amplitude`].
///
/// At the default 240×150 map, a displacement of 8 tiles produces
/// clearly organic boundaries while keeping plate territories
/// visually coherent. Lower values make boundaries straighter;
/// higher values can cause slivers and disconnected plate fragments.
const DEFAULT_BOUNDARY_PERTURBATION_AMPLITUDE: f32 = 8.0;

/// Upper bound on
/// [`TectonicsParams::boundary_perturbation_amplitude`].
///
/// At amplitudes above roughly half the plate spacing, the perturbed
/// lookup can reach a third plate's territory, causing isolated tile
/// fragments and disconnected regions. The plate spacing for 10 plates
/// on a 240×150 map is approximately 34 tiles (the Poisson disk
/// radius), so the maximum is set conservatively at 20.
pub const MAX_BOUNDARY_PERTURBATION_AMPLITUDE: f32 = 20.0;

// ------------- Parameter struct -------------

/// Tuning parameters for the tectonics stage (design §1.9).
///
/// All fields are exposed at the [`crate::TuningParams`] tier; future
/// Basic and Advanced tier presets project a smaller subset onto
/// their own preset structures.
///
/// # Stability of values
///
/// Field defaults define the reference output for the determinism
/// test suite: changing a default is a generation-output change and
/// breaks every committed reference hash. Treat default changes the
/// same as algorithmic changes — they require an intentional
/// reference-hash refresh.
#[derive(Debug, Clone, PartialEq)]
pub struct TectonicsParams {
    /// Target number of tectonic plates the Poisson-disk seed stage
    /// should produce.
    ///
    /// The achieved count is approximate — Poisson-disk sampling
    /// fills the map until no more candidates fit and stops, so the
    /// realised count can be off by one or two from the target. The
    /// minimum-spacing radius is derived from this value and the map
    /// area so that plate count is roughly invariant to map size:
    /// the same target produces the same count on a small map as on
    /// a huge one.
    ///
    /// Range: [`MIN_TARGET_PLATE_COUNT`]..=[`MAX_TARGET_PLATE_COUNT`].
    /// Default: 10.
    pub target_plate_count: u8,

    /// Number of distinct height levels assigned to land tiles, not
    /// counting the fixed ocean, coast, and mountain levels.
    ///
    /// The total number of discrete height levels in the world is
    /// `land_level_count + 3` (ocean + coast + N land + mountain).
    /// Range: [`MIN_LAND_LEVEL_COUNT`]..=[`MAX_LAND_LEVEL_COUNT`].
    /// Default: 3 (giving a six-level world).
    pub land_level_count: u8,

    /// Quantile-target proportions for the discrete height bands.
    ///
    /// Must have exactly `land_level_count + 3` entries summing to
    /// `1.0 ± 1e-4`. Order: index 0 is
    /// ocean, index 1 is coast, indices `2..=land_level_count+1` are
    /// the land levels in ascending order (lowland first), and the
    /// final index is mountain.
    ///
    /// The quantile-threshold pass in step 9 computes the elevation
    /// cutoffs that hit these proportions exactly across the
    /// continuous elevation field. As a corollary, the entry at
    /// index 0 is the achieved water fraction, regardless of how the
    /// underlying tectonic structure shifted the raw elevation
    /// distribution.
    pub level_proportions: Vec<f32>,

    /// Maximum displacement (in tiles) applied to each tile's lookup
    /// position before the nearest-seed search.
    ///
    /// The displacement is derived from a low-frequency `OpenSimplex`
    /// noise field evaluated in 3D wrap-aware space (see
    /// `stages/tectonics/perturbation.rs`). At `0.0` the partition is
    /// a straight Voronoi diagram; at the default value the boundaries
    /// become organic and geographically plausible.
    ///
    /// Range: `0.0..=`[`MAX_BOUNDARY_PERTURBATION_AMPLITUDE`].
    /// Default: 8.0 tiles.
    pub boundary_perturbation_amplitude: f32,
}

impl TectonicsParams {
    /// Returns the default tectonics parameters.
    ///
    /// `Default` cannot be `const` because [`Self::level_proportions`]
    /// owns a `Vec`; the [`Default`] impl below delegates here.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            target_plate_count: DEFAULT_TARGET_PLATE_COUNT,
            land_level_count: DEFAULT_LAND_LEVEL_COUNT,
            level_proportions: DEFAULT_LEVEL_PROPORTIONS.to_vec(),
            boundary_perturbation_amplitude: DEFAULT_BOUNDARY_PERTURBATION_AMPLITUDE,
        }
    }

    /// Validates field ranges and the [`Self::level_proportions`]
    /// invariant.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if:
    /// - `target_plate_count` is outside
    ///   [`MIN_TARGET_PLATE_COUNT`]..=[`MAX_TARGET_PLATE_COUNT`],
    /// - `land_level_count` is outside
    ///   [`MIN_LAND_LEVEL_COUNT`]..=[`MAX_LAND_LEVEL_COUNT`],
    /// - `level_proportions.len()` does not equal `land_level_count + 3`,
    /// - any entry of `level_proportions` is negative or non-finite,
    /// - `level_proportions` does not sum to `1.0` within `1e-4`.
    pub fn validate(&self) -> Result<(), GenerationError> {
        if !(MIN_TARGET_PLATE_COUNT..=MAX_TARGET_PLATE_COUNT).contains(&self.target_plate_count) {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "target_plate_count {} outside [{}, {}]",
                    self.target_plate_count, MIN_TARGET_PLATE_COUNT, MAX_TARGET_PLATE_COUNT
                ),
            });
        }
        if !(MIN_LAND_LEVEL_COUNT..=MAX_LAND_LEVEL_COUNT).contains(&self.land_level_count) {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "land_level_count {} outside [{}, {}]",
                    self.land_level_count, MIN_LAND_LEVEL_COUNT, MAX_LAND_LEVEL_COUNT
                ),
            });
        }

        if !self.boundary_perturbation_amplitude.is_finite()
            || self.boundary_perturbation_amplitude < 0.0
            || self.boundary_perturbation_amplitude > MAX_BOUNDARY_PERTURBATION_AMPLITUDE
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "boundary_perturbation_amplitude {} is outside [0.0, {}]",
                    self.boundary_perturbation_amplitude, MAX_BOUNDARY_PERTURBATION_AMPLITUDE
                ),
            });
        }

        // level_proportions must align with land_level_count.
        let expected_len = usize::from(self.land_level_count) + 3;
        if self.level_proportions.len() != expected_len {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "level_proportions has {} entries; expected {} \
                     (land_level_count {} + 3 fixed: ocean, coast, mountain)",
                    self.level_proportions.len(),
                    expected_len,
                    self.land_level_count
                ),
            });
        }

        let mut sum = 0.0_f32;
        for (i, &p) in self.level_proportions.iter().enumerate() {
            if !p.is_finite() || p < 0.0 {
                return Err(GenerationError::InvalidParams {
                    reason: format!("level_proportions[{i}] = {p} is negative or non-finite"),
                });
            }
            sum += p;
        }
        if (sum - 1.0).abs() > LEVEL_PROPORTIONS_SUM_TOLERANCE {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "level_proportions sums to {sum}, expected 1.0 \
                     within ±{LEVEL_PROPORTIONS_SUM_TOLERANCE}"
                ),
            });
        }

        Ok(())
    }

    /// Returns `true` if every field equals its default value.
    ///
    /// Used by the achievement gate.
    #[must_use]
    pub fn is_all_default(&self) -> bool {
        *self == Self::defaults()
    }
}

impl Default for TectonicsParams {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------- Defaults -------------

    mod defaults {
        use super::*;

        #[test]
        fn defaults_validate() {
            TectonicsParams::defaults()
                .validate()
                .expect("defaults must validate");
        }

        #[test]
        fn default_level_proportions_have_expected_length() {
            let params = TectonicsParams::defaults();
            let expected = usize::from(params.land_level_count) + 3;
            assert_eq!(params.level_proportions.len(), expected);
        }

        #[test]
        fn default_level_proportions_sum_to_one() {
            let params = TectonicsParams::defaults();
            let sum: f32 = params.level_proportions.iter().sum();
            assert!((sum - 1.0).abs() < LEVEL_PROPORTIONS_SUM_TOLERANCE);
        }

        #[test]
        fn defaults_are_all_default() {
            assert!(TectonicsParams::defaults().is_all_default());
        }
    }

    // ------------- target_plate_count bounds -------------

    mod target_plate_count {
        use super::*;

        #[test]
        fn rejects_below_min() {
            let params = TectonicsParams {
                target_plate_count: MIN_TARGET_PLATE_COUNT - 1,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_above_max() {
            let params = TectonicsParams {
                target_plate_count: MAX_TARGET_PLATE_COUNT + 1,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_boundary_values() {
            for n in [MIN_TARGET_PLATE_COUNT, MAX_TARGET_PLATE_COUNT] {
                let params = TectonicsParams {
                    target_plate_count: n,
                    ..TectonicsParams::defaults()
                };
                params.validate().expect("boundary value must validate");
            }
        }
    }

    // ------------- level_proportions / land_level_count coupling -------------

    mod level_proportions {
        use super::*;

        #[test]
        fn rejects_mismatched_length() {
            let params = TectonicsParams {
                land_level_count: 4,
                // Defaults are sized for land_level_count = 3 (six entries).
                level_proportions: DEFAULT_LEVEL_PROPORTIONS.to_vec(),
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_sum_below_one() {
            let params = TectonicsParams {
                level_proportions: vec![0.5, 0.1, 0.1, 0.1, 0.1, 0.05], // 0.95
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_negative_entry() {
            let params = TectonicsParams {
                level_proportions: vec![0.4, 0.05, 0.3, 0.2, -0.05, 0.1],
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_nan_entry() {
            let params = TectonicsParams {
                level_proportions: vec![0.35, 0.05, 0.25, 0.20, f32::NAN, 0.05],
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_non_default_level_count_with_matching_proportions() {
            let params = TectonicsParams {
                land_level_count: 2,
                // 2 land + 3 fixed = 5 entries.
                level_proportions: vec![0.4, 0.05, 0.3, 0.20, 0.05],
                ..TectonicsParams::defaults()
            };
            params.validate().expect("must validate");
        }
    }

    // ------------- boundary_perturbation_amplitude -------------

    mod boundary_perturbation_amplitude {
        use super::*;

        #[test]
        fn rejects_negative_amplitude() {
            let params = TectonicsParams {
                boundary_perturbation_amplitude: -1.0,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_amplitude_above_max() {
            let params = TectonicsParams {
                boundary_perturbation_amplitude: MAX_BOUNDARY_PERTURBATION_AMPLITUDE + 1.0,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_nan_amplitude() {
            let params = TectonicsParams {
                boundary_perturbation_amplitude: f32::NAN,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_zero_amplitude() {
            let params = TectonicsParams {
                boundary_perturbation_amplitude: 0.0,
                ..TectonicsParams::defaults()
            };
            params.validate().expect("zero amplitude must validate");
        }

        #[test]
        fn accepts_max_amplitude() {
            let params = TectonicsParams {
                boundary_perturbation_amplitude: MAX_BOUNDARY_PERTURBATION_AMPLITUDE,
                ..TectonicsParams::defaults()
            };
            params.validate().expect("max amplitude must validate");
        }
    }
}
