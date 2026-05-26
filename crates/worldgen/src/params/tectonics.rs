//! Tuning parameters for the tectonic-plate generation stage.
//!
//! Owns every tunable knob consumed by the tectonics pipeline (design
//! §1.9). Stage 3.1 lands in nine implementation steps; the parameter
//! set is declared up front so the layout is stable across the steps,
//! even though the early steps only consume a few of the fields.
//!
//! See `IMPLEMENTATION_PLAN.md` §3.1 for the step-by-step rollout.

use crate::error::GenerationError;
use crate::world_data::MineralCharacter;

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

/// Default value for [`TectonicsParams::convergence_threshold`].
///
/// A dot-product threshold of 0.3 is roughly 15–20 % of the maximum
/// possible approach speed (which is 2 × the maximum motion magnitude,
/// around 3.0). At this level, most boundaries are classified as
/// convergent or divergent rather than transform unless the relative
/// motion is nearly tangential.
const DEFAULT_CONVERGENCE_THRESHOLD: f32 = 0.3;

/// Default base elevation of continental plates (steps 6–7).
///
/// Chosen so that the continental base is comfortably above sea level
/// after the quantile-normalisation pass in step 8. The exact value
/// affects the width of the continental interior but not the final
/// land fraction (which is enforced by `level_proportions`).
const DEFAULT_CONTINENTAL_BASE_ELEVATION: f32 = 0.3;

/// Default base elevation of oceanic plates (steps 6–7).
///
/// The magnitude matches `DEFAULT_CONTINENTAL_BASE_ELEVATION` so that
/// the base elevation has zero mean across a world with equal land and
/// ocean fractions.
const DEFAULT_OCEANIC_BASE_ELEVATION: f32 = -0.3;

/// Default amplitude of intra-plate elevation noise (step 6).
///
/// Low amplitude keeps the interior of each plate relatively flat,
/// letting boundary effects dominate the topographic structure.
const DEFAULT_INTRA_PLATE_NOISE_AMPLITUDE: f32 = 0.05;

/// Default spatial frequency of intra-plate elevation noise (step 6).
///
/// Scales the noise-input coordinates uniformly; higher values produce
/// finer detail within plates. The underlying scale follows the torus
/// convention from `perturbation.rs`.
const DEFAULT_INTRA_PLATE_NOISE_FREQUENCY: f32 = 2.0;

/// Default Gaussian falloff sigma for boundary-effect propagation.
///
/// The standard deviation of the Gaussian that spreads each boundary
/// tile's elevation modifier into surrounding tiles. At `σ = 8` tiles,
/// the effect is ~61 % of peak at 1σ and ~1 % at 3σ (24 tiles).
const DEFAULT_BOUNDARY_FALLOFF_SIGMA: f32 = 8.0;

/// Default elevation boost for continental–continental convergent
/// boundaries (step 6). Produces high mountain ranges.
const DEFAULT_MOUNTAIN_ELEVATION_BOOST: f32 = 1.2;

/// Default elevation boost for the continental side of a
/// continental–oceanic convergent boundary (step 6).
/// Produces coastal mountain ranges.
const DEFAULT_COASTAL_MOUNTAIN_BOOST: f32 = 0.8;

/// Default elevation penalty for the oceanic side of a
/// continental–oceanic convergent boundary (step 6).
/// Produces subduction trenches. Stored as a negative value.
const DEFAULT_SUBDUCTION_TRENCH_PENALTY: f32 = -0.8;

/// Default elevation boost for oceanic–oceanic convergent boundaries
/// (step 6). Produces island arcs.
const DEFAULT_ISLAND_ARC_BOOST: f32 = 0.4;

/// Default elevation penalty for continental–continental divergent
/// boundaries (step 6). Produces rift valleys. Stored as a negative
/// value.
const DEFAULT_RIFT_VALLEY_PENALTY: f32 = -0.4;

/// Default elevation boost for oceanic–oceanic divergent boundaries
/// (step 6). Produces mid-ocean ridges.
const DEFAULT_MID_OCEAN_RIDGE_BOOST: f32 = 0.3;

/// Default amplitude of the multi-octave detail-noise overlay (step 7).
///
/// Added uniformly over the elevation field to break up flat regions.
/// Lower than the plate and boundary signal so it does not dominate.
const DEFAULT_DETAIL_NOISE_AMPLITUDE: f32 = 0.08;

/// Default spatial frequency of the multi-octave detail noise (step 7).
const DEFAULT_DETAIL_NOISE_FREQUENCY: f32 = 4.0;

/// Minimum allowed value for
/// [`TectonicsParams::detail_noise_octaves`].
pub const MIN_DETAIL_NOISE_OCTAVES: u8 = 1;

/// Maximum allowed value for
/// [`TectonicsParams::detail_noise_octaves`].
///
/// Above eight octaves the additional high-frequency content is below
/// the one-tile resolution of the grid.
pub const MAX_DETAIL_NOISE_OCTAVES: u8 = 8;

/// Default number of octaves for the multi-octave detail noise (step 7).
const DEFAULT_DETAIL_NOISE_OCTAVES: u8 = 3;

/// Default influence radius for tectonic boundary proximity in the
/// hills pass (step 9), in tiles.
///
/// Tiles within this hex-distance of a boundary receive a hilliness
/// probability bonus.
const DEFAULT_HILL_INFLUENCE_RADIUS: u32 = 8;

/// Default hilliness noise threshold for tiles near a tectonic
/// boundary (step 9).
///
/// The hill-noise value (`OpenSimplex`, approximately `[−1, 1]`) at a
/// tile is compared against this threshold. Tiles whose noise value
/// exceeds the threshold are classified as hilly. A lower threshold
/// → more hilly tiles near boundaries.
const DEFAULT_HILL_NEAR_BOUNDARY_THRESHOLD: f32 = -0.2;

/// Default hilliness noise threshold for tiles far from any tectonic
/// boundary (step 9).
///
/// Applies to tiles beyond `hill_influence_radius` of any boundary.
const DEFAULT_HILL_INTERIOR_THRESHOLD: f32 = 0.55;

/// Default spatial frequency of the hilliness noise field (step 9).
const DEFAULT_HILL_NOISE_FREQUENCY: f32 = 5.0;

/// Default minimum magnitude of a plate motion vector.
///
/// Motion vectors are drawn with a magnitude uniformly distributed in
/// `[motion_magnitude_min, motion_magnitude_max]`. The magnitude scale
/// has no physical unit — only relative magnitudes between adjacent
/// plates matter for boundary classification.
const DEFAULT_MOTION_MAGNITUDE_MIN: f32 = 0.5;

/// Default maximum magnitude of a plate motion vector.
///
/// See [`DEFAULT_MOTION_MAGNITUDE_MIN`]. The 3:1 max-to-min ratio
/// produces enough spread that adjacent plates have meaningfully
/// different speeds, driving a useful mix of boundary types, without
/// allowing any one plate to completely dominate its neighbors.
const DEFAULT_MOTION_MAGNITUDE_MAX: f32 = 1.5;

/// Weights for the uniform mineral character distribution (step 4).
///
/// One entry per [`MineralCharacter`] variant, in the order defined
/// by [`MineralCharacter::ALL`]. Higher values make that character
/// more common. Values are relative — they are normalised internally
/// before sampling.
///
/// Default distribution: Ferrous and Industrial are common (4 parts
/// each), Hydrocarbons are moderately common (3 parts), Precious is
/// uncommon (2 parts), and Gems are rare (1 part).
const DEFAULT_MINERAL_WEIGHTS: [f32; 5] = [4.0, 2.0, 1.0, 3.0, 4.0];

/// Default noise threshold for mountain pass demotion (step 10).
///
/// Tiles above this threshold (and passing the optional neighbor filter)
/// are demoted from mountain to high-land. At `0.15`, approximately
/// 35–40 % of interior mountain tiles become passes. Set above `1.0`
/// to disable the pass entirely (no `OpenSimplex` sample exceeds `1.0`).
const DEFAULT_MOUNTAIN_PASS_THRESHOLD: f32 = 0.15;

/// Default spatial frequency for the mountain pass noise field (step 10).
///
/// Higher values produce more frequent, shorter gaps; lower values
/// produce fewer, wider passes. The default of `3.0` gives roughly
/// 3–5 passes per mountain range on a standard 240×150 map.
const DEFAULT_MOUNTAIN_PASS_NOISE_FREQUENCY: f32 = 3.0;

/// Default minimum mountain-neighbor count for neighbor-aware pass
/// carving (step 10).
///
/// When `mountain_pass_neighbor_aware` is `true`, a tile must have at
/// least this many mountain-level neighbors to be eligible for demotion.
/// Three neighbors (out of six) means the tile is embedded in the range
/// interior rather than sitting on its edge.
const DEFAULT_MOUNTAIN_PASS_MIN_MOUNTAIN_NEIGHBORS: u8 = 3;

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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// `1.0 ± LEVEL_PROPORTIONS_SUM_TOLERANCE`. Order: index 0 is
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

    /// Minimum magnitude of a plate motion vector (step 3).
    ///
    /// Motion vector magnitudes are drawn uniformly from
    /// `[motion_magnitude_min, motion_magnitude_max]`. The scale is
    /// dimensionless — only relative magnitudes between adjacent
    /// plates affect boundary classification.
    ///
    /// Must be `> 0.0` and `≤ motion_magnitude_max`.
    /// Default: 0.5.
    pub motion_magnitude_min: f32,

    /// Maximum magnitude of a plate motion vector (step 4).
    ///
    /// See [`Self::motion_magnitude_min`]. A wider range produces more
    /// varied boundary types; a narrow range (min ≈ max) produces
    /// mostly transform boundaries because plates move at similar
    /// speeds.
    ///
    /// Must be `≥ motion_magnitude_min`.
    /// Default: 1.5.
    pub motion_magnitude_max: f32,

    /// Relative weights for sampling [`MineralCharacter`] variants
    /// (step 4).
    ///
    /// Must have exactly one entry per [`MineralCharacter`] variant,
    /// in the order defined by `MineralCharacter::ALL`. All entries
    /// must be finite and `≥ 0.0`; at least one must be `> 0.0`.
    /// Values are relative — they are normalised before sampling, so
    /// `[1.0, 1.0]` and `[4.0, 4.0]` are equivalent.
    ///
    /// Default: Ferrous 4, Precious 2, Gems 1, Hydrocarbons 3,
    /// Industrial 4 — Gems rare, Precious uncommon, others common.
    pub mineral_weights: Vec<f32>,

    // ------------- Step 5: boundary classification -------------
    /// Minimum absolute approach velocity to classify as convergent or
    /// divergent (rather than transform) in step 5.
    ///
    /// The approach velocity is `dot(motion_A − motion_B, normal_A→B)`.
    /// If the absolute value is above this threshold the boundary is
    /// convergent (positive) or divergent (negative); otherwise it is
    /// transform.
    ///
    /// Range: `0.0..=∞` (finite). Default: 0.3.
    /// Raising toward `motion_magnitude_max` produces more transform
    /// boundaries; `0.0` makes every boundary convergent or divergent.
    pub convergence_threshold: f32,

    // ------------- Steps 6–7: elevation field -------------
    /// Base elevation assigned to tiles on continental plates, before
    /// noise and boundary effects are applied (step 6).
    ///
    /// Must be finite. Default: 0.3.
    pub continental_base_elevation: f32,

    /// Base elevation assigned to tiles on oceanic plates, before
    /// noise and boundary effects are applied (step 6).
    ///
    /// Must be finite. Default: −0.3.
    pub oceanic_base_elevation: f32,

    /// Amplitude of the intra-plate noise overlay added on top of the
    /// per-plate base elevation (step 6).
    ///
    /// Must be finite and `≥ 0.0`. Default: 0.05.
    pub intra_plate_noise_amplitude: f32,

    /// Spatial frequency multiplier for the intra-plate noise (step 6).
    ///
    /// Scales the noise-input coordinates; `1.0` matches the spatial
    /// frequency of `boundary_perturbation_amplitude`. Must be finite
    /// and `> 0.0`. Default: 2.0.
    pub intra_plate_noise_frequency: f32,

    /// Standard deviation of the Gaussian used to propagate boundary
    /// elevation modifiers into surrounding tiles (step 6), in tiles.
    ///
    /// Effects fall to ~1 % of peak at 3σ. Must be finite and `> 0.0`.
    /// Default: 8.0 tiles.
    pub boundary_falloff_sigma: f32,

    /// Elevation modifier at continental–continental convergent
    /// boundaries (step 6). Produces high mountain ranges.
    ///
    /// Must be finite. Default: 1.2.
    pub mountain_elevation_boost: f32,

    /// Elevation modifier for the continental side of a
    /// continental–oceanic convergent boundary (step 6).
    /// Produces coastal mountain ranges.
    ///
    /// Must be finite. Default: 0.8.
    pub coastal_mountain_boost: f32,

    /// Elevation modifier for the oceanic side of a
    /// continental–oceanic convergent boundary (step 6).
    /// Produces subduction trenches. Typically negative.
    ///
    /// Must be finite. Default: −0.8.
    pub subduction_trench_penalty: f32,

    /// Elevation modifier at oceanic–oceanic convergent boundaries
    /// (step 6). Produces island arcs.
    ///
    /// Must be finite. Default: 0.4.
    pub island_arc_boost: f32,

    /// Elevation modifier at continental–continental divergent
    /// boundaries (step 6). Produces rift valleys. Typically negative.
    ///
    /// Must be finite. Default: −0.4.
    pub rift_valley_penalty: f32,

    /// Elevation modifier at oceanic–oceanic divergent boundaries
    /// (step 6). Produces mid-ocean ridges.
    ///
    /// Must be finite. Default: 0.3.
    pub mid_ocean_ridge_boost: f32,

    /// Amplitude of the multi-octave detail-noise overlay added over
    /// the full elevation field (step 7).
    ///
    /// Must be finite and `≥ 0.0`. Default: 0.08.
    pub detail_noise_amplitude: f32,

    /// Spatial frequency multiplier for the detail-noise overlay
    /// (step 7).
    ///
    /// Must be finite and `> 0.0`. Default: 4.0.
    pub detail_noise_frequency: f32,

    /// Number of octaves for the detail-noise overlay (step 7).
    ///
    /// Each octave doubles the frequency and halves the amplitude.
    /// Range: [`MIN_DETAIL_NOISE_OCTAVES`]..=[`MAX_DETAIL_NOISE_OCTAVES`].
    /// Default: 3.
    pub detail_noise_octaves: u8,

    // ------------- Step 9: hills -------------
    /// Hex-distance within which proximity to a tectonic boundary
    /// increases hill probability (step 9).
    ///
    /// Tiles within this radius of a boundary tile have their hilliness
    /// threshold linearly interpolated toward `hill_near_boundary_threshold`.
    /// Default: 8 tiles.
    pub hill_influence_radius: u32,

    /// Hill-noise threshold applied to tiles at a tectonic boundary
    /// (step 9).
    ///
    /// The `OpenSimplex` hill-noise value (approximately `[−1, 1]`) at
    /// each tile is compared against this threshold. Tiles whose value
    /// exceeds the threshold are classified as hilly. A lower value
    /// → more hilly tiles near boundaries. Must be finite.
    /// Default: −0.2 (≈ 60 % of boundary tiles are hilly).
    pub hill_near_boundary_threshold: f32,

    /// Hill-noise threshold applied to tiles far from any tectonic
    /// boundary (step 9).
    ///
    /// Applied to tiles beyond `hill_influence_radius` of any boundary.
    /// Must be finite. Default: 0.55 (≈ 22 % of interior tiles are hilly).
    pub hill_interior_threshold: f32,

    /// Spatial frequency multiplier for the hilliness noise field
    /// (step 9).
    ///
    /// Higher values produce smaller, more scattered hill patches.
    /// Must be finite and `> 0.0`. Default: 5.0.
    pub hill_noise_frequency: f32,

    /// Noise threshold for mountain pass demotion (step 10).
    ///
    /// After height-level quantization, mountain tiles whose wrap-aware
    /// `OpenSimplex` noise value exceeds this threshold are demoted to
    /// high-land, creating traversable passes through ranges.
    ///
    /// At `0.0` roughly 50 % of qualifying tiles are demoted; at `0.5`
    /// roughly 25 %. Set above `1.0` to disable the pass entirely
    /// (no `OpenSimplex` sample exceeds `1.0`). Must be finite.
    /// Default: 0.15.
    pub mountain_pass_threshold: f32,

    /// Spatial frequency multiplier for the mountain pass noise field
    /// (step 10).
    ///
    /// Higher values produce more frequent, shorter gaps; lower values
    /// produce wider, fewer passes. Must be finite and `> 0.0`. Default: 3.0.
    pub mountain_pass_noise_frequency: f32,

    /// Whether to restrict pass demotion to range-interior tiles (step 10).
    ///
    /// When `true`, a mountain tile is only eligible for demotion if at
    /// least `mountain_pass_min_mountain_neighbors` of its six neighbors
    /// are also at mountain level. This ensures passes cut *through* a
    /// range rather than nibbling its edges. Default: `true`.
    pub mountain_pass_neighbor_aware: bool,

    /// Minimum mountain-neighbor count for neighbor-aware pass carving
    /// (step 10).
    ///
    /// Only consulted when `mountain_pass_neighbor_aware` is `true`. A
    /// tile must have at least this many mountain-level neighbors to be
    /// eligible for demotion. Range: `0`–`6`. Default: 3.
    pub mountain_pass_min_mountain_neighbors: u8,
}

impl TectonicsParams {
    /// Returns the default tectonics parameters.
    ///
    /// `Default` cannot be `const` because [`Self::level_proportions`]
    /// and [`Self::mineral_weights`] own `Vec`s; the [`Default`] impl
    /// below delegates here.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            target_plate_count: DEFAULT_TARGET_PLATE_COUNT,
            land_level_count: DEFAULT_LAND_LEVEL_COUNT,
            level_proportions: DEFAULT_LEVEL_PROPORTIONS.to_vec(),
            motion_magnitude_min: DEFAULT_MOTION_MAGNITUDE_MIN,
            motion_magnitude_max: DEFAULT_MOTION_MAGNITUDE_MAX,
            mineral_weights: DEFAULT_MINERAL_WEIGHTS.to_vec(),
            convergence_threshold: DEFAULT_CONVERGENCE_THRESHOLD,
            continental_base_elevation: DEFAULT_CONTINENTAL_BASE_ELEVATION,
            oceanic_base_elevation: DEFAULT_OCEANIC_BASE_ELEVATION,
            intra_plate_noise_amplitude: DEFAULT_INTRA_PLATE_NOISE_AMPLITUDE,
            intra_plate_noise_frequency: DEFAULT_INTRA_PLATE_NOISE_FREQUENCY,
            boundary_falloff_sigma: DEFAULT_BOUNDARY_FALLOFF_SIGMA,
            mountain_elevation_boost: DEFAULT_MOUNTAIN_ELEVATION_BOOST,
            mountain_pass_threshold: DEFAULT_MOUNTAIN_PASS_THRESHOLD,
            mountain_pass_noise_frequency: DEFAULT_MOUNTAIN_PASS_NOISE_FREQUENCY,
            mountain_pass_neighbor_aware: true,
            mountain_pass_min_mountain_neighbors: DEFAULT_MOUNTAIN_PASS_MIN_MOUNTAIN_NEIGHBORS,
            coastal_mountain_boost: DEFAULT_COASTAL_MOUNTAIN_BOOST,
            subduction_trench_penalty: DEFAULT_SUBDUCTION_TRENCH_PENALTY,
            island_arc_boost: DEFAULT_ISLAND_ARC_BOOST,
            rift_valley_penalty: DEFAULT_RIFT_VALLEY_PENALTY,
            mid_ocean_ridge_boost: DEFAULT_MID_OCEAN_RIDGE_BOOST,
            detail_noise_amplitude: DEFAULT_DETAIL_NOISE_AMPLITUDE,
            detail_noise_frequency: DEFAULT_DETAIL_NOISE_FREQUENCY,
            detail_noise_octaves: DEFAULT_DETAIL_NOISE_OCTAVES,
            hill_influence_radius: DEFAULT_HILL_INFLUENCE_RADIUS,
            hill_near_boundary_threshold: DEFAULT_HILL_NEAR_BOUNDARY_THRESHOLD,
            hill_interior_threshold: DEFAULT_HILL_INTERIOR_THRESHOLD,
            hill_noise_frequency: DEFAULT_HILL_NOISE_FREQUENCY,
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
    /// - `level_proportions` does not sum to `1.0` within
    ///   `LEVEL_PROPORTIONS_SUM_TOLERANCE`.
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
        self.validate_level_proportions()?;
        self.validate_motion_and_minerals()?;
        self.validate_elevation_and_hill_params()
    }

    fn validate_level_proportions(&self) -> Result<(), GenerationError> {
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

    fn validate_motion_and_minerals(&self) -> Result<(), GenerationError> {
        if !self.motion_magnitude_min.is_finite() || self.motion_magnitude_min <= 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "motion_magnitude_min {} must be finite and > 0.0",
                    self.motion_magnitude_min
                ),
            });
        }
        if !self.motion_magnitude_max.is_finite()
            || self.motion_magnitude_max < self.motion_magnitude_min
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "motion_magnitude_max {} must be finite and \
                     >= motion_magnitude_min {}",
                    self.motion_magnitude_max, self.motion_magnitude_min
                ),
            });
        }
        let expected_weights = MineralCharacter::ALL.len();
        if self.mineral_weights.len() != expected_weights {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "mineral_weights has {} entries; expected {} \
                     (one per MineralCharacter variant)",
                    self.mineral_weights.len(),
                    expected_weights
                ),
            });
        }
        let weight_sum: f32 = self.mineral_weights.iter().sum();
        for (i, &w) in self.mineral_weights.iter().enumerate() {
            if !w.is_finite() || w < 0.0 {
                return Err(GenerationError::InvalidParams {
                    reason: format!("mineral_weights[{i}] = {w} is negative or non-finite"),
                });
            }
        }
        if weight_sum <= 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: "mineral_weights must have at least one entry > 0.0".to_owned(),
            });
        }
        Ok(())
    }

    fn validate_elevation_and_hill_params(&self) -> Result<(), GenerationError> {
        if !self.convergence_threshold.is_finite() || self.convergence_threshold < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "convergence_threshold {} must be finite and >= 0.0",
                    self.convergence_threshold
                ),
            });
        }
        for (name, value) in [
            (
                "continental_base_elevation",
                self.continental_base_elevation,
            ),
            ("oceanic_base_elevation", self.oceanic_base_elevation),
            ("mountain_elevation_boost", self.mountain_elevation_boost),
            ("coastal_mountain_boost", self.coastal_mountain_boost),
            ("subduction_trench_penalty", self.subduction_trench_penalty),
            ("island_arc_boost", self.island_arc_boost),
            ("rift_valley_penalty", self.rift_valley_penalty),
            ("mid_ocean_ridge_boost", self.mid_ocean_ridge_boost),
            ("detail_noise_amplitude", self.detail_noise_amplitude),
            (
                "hill_near_boundary_threshold",
                self.hill_near_boundary_threshold,
            ),
            ("hill_interior_threshold", self.hill_interior_threshold),
        ] {
            if !value.is_finite() {
                return Err(GenerationError::InvalidParams {
                    reason: format!("{name} = {value} is not finite"),
                });
            }
        }
        if !self.intra_plate_noise_amplitude.is_finite() || self.intra_plate_noise_amplitude < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "intra_plate_noise_amplitude {} must be finite and >= 0.0",
                    self.intra_plate_noise_amplitude
                ),
            });
        }
        for (name, value) in [
            (
                "intra_plate_noise_frequency",
                self.intra_plate_noise_frequency,
            ),
            ("boundary_falloff_sigma", self.boundary_falloff_sigma),
            ("detail_noise_frequency", self.detail_noise_frequency),
            ("hill_noise_frequency", self.hill_noise_frequency),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(GenerationError::InvalidParams {
                    reason: format!("{name} = {value} must be finite and > 0.0"),
                });
            }
        }

        if !self.mountain_pass_threshold.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "mountain_pass_threshold {} must be finite",
                    self.mountain_pass_threshold
                ),
            });
        }
        if !self.mountain_pass_noise_frequency.is_finite()
            || self.mountain_pass_noise_frequency <= 0.0
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "mountain_pass_noise_frequency {} must be finite and > 0.0",
                    self.mountain_pass_noise_frequency
                ),
            });
        }
        if self.mountain_pass_min_mountain_neighbors > 6 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "mountain_pass_min_mountain_neighbors {} exceeds maximum of 6",
                    self.mountain_pass_min_mountain_neighbors
                ),
            });
        }

        if !(MIN_DETAIL_NOISE_OCTAVES..=MAX_DETAIL_NOISE_OCTAVES)
            .contains(&self.detail_noise_octaves)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "detail_noise_octaves {} outside [{}, {}]",
                    self.detail_noise_octaves, MIN_DETAIL_NOISE_OCTAVES, MAX_DETAIL_NOISE_OCTAVES
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

    // ------------- motion magnitude -------------

    mod motion_magnitude {
        use super::*;

        #[test]
        fn rejects_zero_min() {
            let params = TectonicsParams {
                motion_magnitude_min: 0.0,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_negative_min() {
            let params = TectonicsParams {
                motion_magnitude_min: -0.1,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_max_below_min() {
            let params = TectonicsParams {
                motion_magnitude_min: 1.0,
                motion_magnitude_max: 0.5,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_equal_min_max() {
            // Degenerate but valid: all plates get the same speed.
            let params = TectonicsParams {
                motion_magnitude_min: 1.0,
                motion_magnitude_max: 1.0,
                ..TectonicsParams::defaults()
            };
            params.validate().expect("equal min/max must validate");
        }
    }

    // ------------- mineral weights -------------

    mod mineral_weights {
        use super::*;

        #[test]
        fn rejects_wrong_length() {
            let params = TectonicsParams {
                mineral_weights: vec![1.0, 1.0], // too few
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_all_zero_weights() {
            let params = TectonicsParams {
                mineral_weights: vec![0.0; MineralCharacter::ALL.len()],
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_negative_weight() {
            let mut weights = DEFAULT_MINERAL_WEIGHTS.to_vec();
            weights[0] = -1.0;
            let params = TectonicsParams {
                mineral_weights: weights,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_single_nonzero_weight() {
            // Only one type available — valid, just degenerate.
            let mut weights = vec![0.0; MineralCharacter::ALL.len()];
            weights[0] = 1.0;
            let params = TectonicsParams {
                mineral_weights: weights,
                ..TectonicsParams::defaults()
            };
            params
                .validate()
                .expect("single nonzero weight must validate");
        }
    }

    mod mountain_pass_params {
        use super::*;

        #[test]
        fn rejects_nan_threshold() {
            let params = TectonicsParams {
                mountain_pass_threshold: f32::NAN,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_threshold_above_one_disables_pass() {
            // Threshold > 1.0 is valid — it's the "disabled" sentinel.
            let params = TectonicsParams {
                mountain_pass_threshold: 1.5,
                ..TectonicsParams::defaults()
            };
            params
                .validate()
                .expect("threshold > 1.0 (disabled) must validate");
        }

        #[test]
        fn accepts_negative_threshold_demotes_all() {
            let params = TectonicsParams {
                mountain_pass_threshold: -0.9,
                ..TectonicsParams::defaults()
            };
            params.validate().expect("negative threshold must validate");
        }

        #[test]
        fn rejects_zero_pass_frequency() {
            let params = TectonicsParams {
                mountain_pass_noise_frequency: 0.0,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_nan_pass_frequency() {
            let params = TectonicsParams {
                mountain_pass_noise_frequency: f32::NAN,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_min_neighbors_above_six() {
            let params = TectonicsParams {
                mountain_pass_min_mountain_neighbors: 7,
                ..TectonicsParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_min_neighbors_at_six() {
            let params = TectonicsParams {
                mountain_pass_min_mountain_neighbors: 6,
                ..TectonicsParams::defaults()
            };
            params.validate().expect("6 neighbors must validate");
        }

        #[test]
        fn accepts_min_neighbors_zero() {
            let params = TectonicsParams {
                mountain_pass_min_mountain_neighbors: 0,
                ..TectonicsParams::defaults()
            };
            params.validate().expect("0 neighbors must validate");
        }
    }
}
