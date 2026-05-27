//! Tuning parameters for the biome stage (Stage 3.4, design §1.12).
//!
//! Owns the scalar knobs consumed by Whittaker lookup, hydrology
//! overrides, conservative smoothing, and forest-density generation.

use crate::error::GenerationError;

const DEFAULT_TEMPERATURE_BIN_COUNT: u8 = 8;
const DEFAULT_MOISTURE_BIN_COUNT: u8 = 8;
const DEFAULT_HYDROLOGY_OVERRIDE_STRENGTH: f32 = 0.8;
const DEFAULT_SMOOTHING_PASSES: u8 = 0;
const DEFAULT_FOREST_NOISE_FREQUENCY: f32 = 0.4;
const DEFAULT_FOREST_NOISE_AMPLITUDE: f32 = 0.6;

const MIN_BIN_COUNT: u8 = 2;
const MAX_BIN_COUNT: u8 = 32;
const MAX_SMOOTHING_PASSES: u8 = 5;

/// Tuning parameters for biome assignment and forest-density output.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BiomeParams {
    /// Number of temperature bins expected by the biome table.
    ///
    /// Valid range: `2..=32`. This is not freely sweepable: changing it
    /// requires a biome table with the same grid dimensions. Default: 8.
    pub temperature_bin_count: u8,

    /// Number of moisture bins expected by the biome table.
    ///
    /// Valid range: `2..=32`. This is not freely sweepable: changing it
    /// requires a biome table with the same grid dimensions. Default: 8.
    pub moisture_bin_count: u8,

    /// Strength of hydrology-driven fertile-biome overrides.
    ///
    /// `0.0` disables overrides. `1.0` applies every eligible override.
    /// Intermediate values apply to stronger water influences first.
    /// Valid range: `[0.0, 1.0]`. Default: 0.8.
    pub hydrology_override_strength: f32,

    /// Number of conservative single-tile-island smoothing passes.
    ///
    /// Valid range: `0..=5`. `0` disables smoothing. Default: 0.
    pub smoothing_passes: u8,

    /// Spatial frequency of forest-density noise in map-width units.
    ///
    /// Valid range: `[0.05, 2.0]`. Default: 0.4.
    pub forest_noise_frequency: f32,

    /// Amplitude of forest-density variation around `0.5`.
    ///
    /// Valid range: `[0.0, 1.0]`. Default: 0.6.
    pub forest_noise_amplitude: f32,
}

impl BiomeParams {
    /// Returns the default biome parameters.
    #[must_use]
    pub const fn defaults() -> Self {
        Self {
            temperature_bin_count: DEFAULT_TEMPERATURE_BIN_COUNT,
            moisture_bin_count: DEFAULT_MOISTURE_BIN_COUNT,
            hydrology_override_strength: DEFAULT_HYDROLOGY_OVERRIDE_STRENGTH,
            smoothing_passes: DEFAULT_SMOOTHING_PASSES,
            forest_noise_frequency: DEFAULT_FOREST_NOISE_FREQUENCY,
            forest_noise_amplitude: DEFAULT_FOREST_NOISE_AMPLITUDE,
        }
    }

    /// Validates all field ranges.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if any field falls
    /// outside its documented range.
    pub fn validate(&self) -> Result<(), GenerationError> {
        validate_bin_count("temperature_bin_count", self.temperature_bin_count)?;
        validate_bin_count("moisture_bin_count", self.moisture_bin_count)?;
        if !self.hydrology_override_strength.is_finite()
            || !(0.0..=1.0).contains(&self.hydrology_override_strength)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "hydrology_override_strength {} must be finite and in [0.0, 1.0]",
                    self.hydrology_override_strength
                ),
            });
        }
        if self.smoothing_passes > MAX_SMOOTHING_PASSES {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "smoothing_passes {} must be in 0..={MAX_SMOOTHING_PASSES}",
                    self.smoothing_passes
                ),
            });
        }
        if !self.forest_noise_frequency.is_finite()
            || !(0.05..=2.0).contains(&self.forest_noise_frequency)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "forest_noise_frequency {} must be finite and in [0.05, 2.0]",
                    self.forest_noise_frequency
                ),
            });
        }
        if !self.forest_noise_amplitude.is_finite()
            || !(0.0..=1.0).contains(&self.forest_noise_amplitude)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "forest_noise_amplitude {} must be finite and in [0.0, 1.0]",
                    self.forest_noise_amplitude
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

impl Default for BiomeParams {
    fn default() -> Self {
        Self::defaults()
    }
}

fn validate_bin_count(name: &str, value: u8) -> Result<(), GenerationError> {
    if !(MIN_BIN_COUNT..=MAX_BIN_COUNT).contains(&value) {
        return Err(GenerationError::InvalidParams {
            reason: format!("{name} {value} must be in {MIN_BIN_COUNT}..={MAX_BIN_COUNT}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        BiomeParams::defaults()
            .validate()
            .expect("defaults must always validate");
    }

    #[test]
    fn rejects_zero_override_strength_underflow() {
        let params = BiomeParams {
            hydrology_override_strength: -0.1,
            ..BiomeParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }

    #[test]
    fn rejects_too_many_smoothing_passes() {
        let params = BiomeParams {
            smoothing_passes: MAX_SMOOTHING_PASSES + 1,
            ..BiomeParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }
}
