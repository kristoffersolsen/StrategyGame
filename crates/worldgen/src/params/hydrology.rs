//! Tuning parameters for the hydrology stage (Stage 3.3, design §1.11).
//!
//! Owns every tunable knob consumed by drainage, lake detection, river
//! classification, floodplain marking, and canyon carving.

use crate::error::GenerationError;

// ------------- Default values -------------

/// Default basin area at which an inland depression becomes a lake.
const DEFAULT_LAKE_BASIN_AREA_THRESHOLD: u32 = 50;

/// Default normalised flow needed for the smallest river tier.
const DEFAULT_RIVER_FLOW_THRESHOLD: f32 = 0.15;

/// Default normalised flow needed for a major river.
const DEFAULT_MAJOR_RIVER_FLOW_THRESHOLD: f32 = 0.60;

/// Default elevation tolerance for floodplain marking.
const DEFAULT_FLOODPLAIN_ELEVATION_TOLERANCE: f32 = 0.05;

/// Default stream-power threshold for canyon carving.
const DEFAULT_CANYON_FLOW_SLOPE_THRESHOLD: f32 = 0.25;

/// Default number of height levels removed by canyon carving.
const DEFAULT_CANYON_DEPTH: u8 = 1;

/// Maximum accepted canyon carving depth.
const MAX_CANYON_DEPTH: u8 = 2;

// ------------- Parameter struct -------------

/// Tuning parameters for the hydrology stage.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HydrologyParams {
    /// Basin tile count at or above which a sink becomes a lake.
    ///
    /// Smaller basins are filled to their spill point instead. Must be
    /// greater than zero. Default: 50.
    pub lake_basin_area_threshold: u32,

    /// Normalised accumulated flow threshold for river creation.
    ///
    /// Tiles with flow below this threshold are not rivers. Must be
    /// finite and in `[0.0, 1.0]`. Default: 0.15.
    pub river_flow_threshold: f32,

    /// Normalised accumulated flow threshold for major rivers.
    ///
    /// Must be finite, in `[0.0, 1.0]`, and at least
    /// [`Self::river_flow_threshold`]. Default: 0.60.
    pub major_river_flow_threshold: f32,

    /// Maximum elevation difference from an adjacent river tile for
    /// floodplain marking.
    ///
    /// Must be finite and `≥ 0.0`. Default: 0.05.
    pub floodplain_elevation_tolerance: f32,

    /// Minimum `flow × slope` product required to carve a canyon.
    ///
    /// Must be finite and `≥ 0.0`. Default: 0.25.
    pub canyon_flow_slope_threshold: f32,

    /// Number of discrete height levels removed by canyon carving.
    ///
    /// Valid range: `1..=2`. Default: 1.
    pub canyon_depth: u8,
}

impl HydrologyParams {
    /// Returns the default hydrology parameters.
    #[must_use]
    pub const fn defaults() -> Self {
        Self {
            lake_basin_area_threshold: DEFAULT_LAKE_BASIN_AREA_THRESHOLD,
            river_flow_threshold: DEFAULT_RIVER_FLOW_THRESHOLD,
            major_river_flow_threshold: DEFAULT_MAJOR_RIVER_FLOW_THRESHOLD,
            floodplain_elevation_tolerance: DEFAULT_FLOODPLAIN_ELEVATION_TOLERANCE,
            canyon_flow_slope_threshold: DEFAULT_CANYON_FLOW_SLOPE_THRESHOLD,
            canyon_depth: DEFAULT_CANYON_DEPTH,
        }
    }

    /// Validates all field ranges.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if any field falls
    /// outside its documented range.
    pub fn validate(&self) -> Result<(), GenerationError> {
        if self.lake_basin_area_threshold == 0 {
            return Err(GenerationError::InvalidParams {
                reason: "lake_basin_area_threshold must be greater than 0".to_owned(),
            });
        }
        if !self.river_flow_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.river_flow_threshold)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "river_flow_threshold {} must be finite and in [0.0, 1.0]",
                    self.river_flow_threshold
                ),
            });
        }
        if !self.major_river_flow_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.major_river_flow_threshold)
            || self.major_river_flow_threshold < self.river_flow_threshold
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "major_river_flow_threshold {} must be finite, in [0.0, 1.0], \
                     and >= river_flow_threshold {}",
                    self.major_river_flow_threshold, self.river_flow_threshold
                ),
            });
        }
        if !self.floodplain_elevation_tolerance.is_finite()
            || self.floodplain_elevation_tolerance < 0.0
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "floodplain_elevation_tolerance {} must be finite and >= 0.0",
                    self.floodplain_elevation_tolerance
                ),
            });
        }
        if !self.canyon_flow_slope_threshold.is_finite() || self.canyon_flow_slope_threshold < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "canyon_flow_slope_threshold {} must be finite and >= 0.0",
                    self.canyon_flow_slope_threshold
                ),
            });
        }
        if !(1..=MAX_CANYON_DEPTH).contains(&self.canyon_depth) {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "canyon_depth {} must be in 1..={MAX_CANYON_DEPTH}",
                    self.canyon_depth
                ),
            });
        }
        Ok(())
    }

    /// Returns `true` if every field equals its default value.
    #[must_use]
    pub fn is_all_default(&self) -> bool {
        *self == Self::defaults()
    }
}

impl Default for HydrologyParams {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        HydrologyParams::defaults()
            .validate()
            .expect("defaults must always validate");
    }

    #[test]
    fn rejects_major_threshold_below_river_threshold() {
        let params = HydrologyParams {
            river_flow_threshold: 0.6,
            major_river_flow_threshold: 0.2,
            ..HydrologyParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }

    #[test]
    fn rejects_canyon_depth_above_cap() {
        let params = HydrologyParams {
            canyon_depth: MAX_CANYON_DEPTH + 1,
            ..HydrologyParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }
}
