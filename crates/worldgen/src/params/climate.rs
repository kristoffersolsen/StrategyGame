//! Tuning parameters for the climate stage (Stage 3.2, design §1.10).
//!
//! Owns every tunable knob consumed by the climate pipeline: temperature
//! field construction, wind field construction, moisture evaporation,
//! advection, and orographic precipitation.
//!
//! See `documents/crates/worldgen/parameters/climate.md` for the
//! parameter reference and `documents/crates/worldgen/pipeline.md` for
//! the stage pipeline.

use crate::error::GenerationError;

// ------------- CFL hard cap -------------

/// Maximum wind amplitude enforced at parameter validation time.
///
/// The donor-cell upwind advection scheme requires that the total
/// outflow fraction from any tile per iteration is ≤ 1. For a hex
/// grid, the sum of positive dot-products of the wind vector against
/// all six unit neighbour vectors is at most `2 |v|`. The strict CFL
/// bound is therefore `2|v| ≤ 1`, i.e., `|v| ≤ 0.5`. We use a
/// conservative safety margin of `0.25` — half the strict bound —
/// documented in ADR-0005.
pub const CFL_MAX_WIND_AMPLITUDE: f32 = 0.25;

/// Maximum allowed isotropic diffusion rate per advection iteration.
///
/// With [`CFL_MAX_WIND_AMPLITUDE`] limiting wind outflow to at most
/// `0.5`, diffusion must be no more than `0.5` so total outflow cannot
/// exceed the tile's current moisture.
pub(crate) const MAX_DIFFUSION_RATE: f32 = 0.5;

// ------------- Default values -------------

/// Default normalised temperature at the equator.
///
/// Dimensionless. The temperature field is normalised so that this
/// value represents the warmest point on the map. Downstream stages
/// (biomes) compare temperature to thresholds expressed in the same
/// units.
const DEFAULT_EQUATOR_TEMPERATURE: f32 = 1.0;

/// Default normalised temperature at the poles.
///
/// Must be less than [`DEFAULT_EQUATOR_TEMPERATURE`]. The gap between
/// the two drives the strength of the temperature gradient and
/// therefore the width of biome bands.
const DEFAULT_POLE_TEMPERATURE: f32 = -0.5;

/// Default temperature drop per unit of elevation above sea level.
///
/// Applied to tiles with positive elevation (above sea level). The
/// drop is proportional to the raw continuous elevation value, so the
/// effective magnitude depends on the elevation field's scale.
const DEFAULT_ELEVATION_LAPSE_RATE: f32 = 0.25;

/// Default global temperature shift from the advanced-layer climate band.
///
/// Zero at the tuning layer; the advanced-layer `ClimateBand` enum
/// applies an additional offset on top of this when preparing params
/// for the game crate.
const DEFAULT_CLIMATE_BAND_OFFSET: f32 = 0.0;

/// Default zonal wind amplitude `A` in `u(φ) = −A·cos(3π·φ)`.
///
/// Must satisfy the CFL hard cap: `wind_amplitude ≤`
/// [`CFL_MAX_WIND_AMPLITUDE`]. At the default of `0.18`, the three
/// climate bands (trade winds, westerlies, polar easterlies) are
/// clearly visible without numerical instability.
const DEFAULT_WIND_AMPLITUDE: f32 = 0.18;

/// Default amplitude of the low-frequency wind noise perturbation.
///
/// Added to both the x and y components of the wind field to break
/// up the uniform zonal pattern. Values above ≈ 0.1 may produce
/// visible stripe artifacts at low `wind_amplitude`.
const DEFAULT_WIND_NOISE_AMPLITUDE: f32 = 0.04;

/// Default spatial frequency of the wind noise perturbation.
///
/// Expressed in fractions of the map width; `0.5` means the noise
/// completes about half a cycle across the map, producing continent-scale
/// deflections.
const DEFAULT_WIND_NOISE_FREQUENCY: f32 = 0.5;

/// Default per-iteration evaporation rate from water tiles.
///
/// Each water tile adds this fraction of its maximum moisture capacity
/// to the moisture field per iteration. Temperature scales the actual
/// rate; cold water tiles evaporate more slowly.
const DEFAULT_EVAPORATION_RATE: f32 = 0.05;

/// Default temperature floor below which evaporation is zero.
///
/// Tiles whose temperature falls below this threshold contribute no
/// evaporation. The floor prevents frozen polar oceans from acting as
/// moisture sources.
const DEFAULT_EVAPORATION_TEMP_FLOOR: f32 = -0.2;

/// Default linear temperature scaling factor for evaporation.
///
/// Multiplies the distance above `evaporation_temp_floor` to produce
/// the evaporation rate fraction. Higher values produce stronger
/// temperature dependence.
const DEFAULT_EVAPORATION_TEMP_SCALE: f32 = 1.0;

/// Default fraction of moisture precipitated per iteration in flat terrain.
///
/// Applied uniformly to all land tiles before orographic and temperature
/// factors are multiplied in. Raising this increases total precipitation
/// but reduces how far moisture travels inland.
const DEFAULT_BASE_PRECIPITATION_RATE: f32 = 0.02;

/// Default sigmoid offset for air moisture-holding capacity vs temperature.
///
/// The capacity sigmoid `σ((temp − offset) × scale)` reaches 0.5 when
/// `temp == offset`. Raising this toward the equator temperature makes
/// cold air carry less moisture, strengthening the effect of cold
/// temperatures on precipitation.
const DEFAULT_TEMP_CAPACITY_OFFSET: f32 = 0.5;

/// Default sigmoid steepness for the temperature–capacity curve.
///
/// Higher values produce a sharper transition between low and high
/// moisture-holding capacity. At `4.0`, the capacity drops to 10 %
/// about one temperature unit below the offset.
const DEFAULT_TEMP_CAPACITY_SCALE: f32 = 4.0;

/// Default multiplier on positive elevation gain for orographic precipitation.
///
/// When a tile is higher than its upwind neighbour, precipitation is
/// multiplied by `1 + orographic_factor × max(0, Δelev)`. Larger
/// values produce stronger rain shadows behind mountain ranges.
const DEFAULT_OROGRAPHIC_FACTOR: f32 = 1.5;

/// Default hard cap on the fraction of moisture precipitated per tile
/// per iteration.
///
/// Prevents numerical blow-up when the orographic factor or temperature
/// capacity would otherwise demand more moisture than the tile holds.
const DEFAULT_MAX_PRECIPITATION_FRACTION: f32 = 0.6;

/// Default multiplier on `map_width` to derive the iteration count.
///
/// The iteration count is `max(iteration_count_min,
/// round(iteration_count_factor × map_width))`. At `1.5`, a 240-wide
/// map runs 360 advection iterations, enough for moisture to complete
/// 1.5 full transits of the map.
const DEFAULT_ITERATION_COUNT_FACTOR: f32 = 1.5;

/// Default minimum iteration count for small maps.
///
/// Prevents very small maps from reaching steady state too quickly with
/// too few iterations for latitude bands to become visible.
const DEFAULT_ITERATION_COUNT_MIN: u32 = 100;

/// Default coastal moisture contribution per adjacent water tile per iteration.
///
/// Zero by default for backwards compatibility. A value of 0.02–0.05 models
/// sea-breeze and coastal fog: land tiles immediately bordering ocean or coast
/// receive direct moisture regardless of wind direction.
const DEFAULT_COASTAL_MOISTURE_FACTOR: f32 = 0.0;

/// Default fraction of a land tile's moisture re-emitted per iteration.
///
/// Zero by default for backwards compatibility. A value of 0.05–0.15 models
/// plant transpiration and soil evaporation: wet land tiles recycle a fraction
/// of their moisture back into the atmosphere, allowing it to travel further
/// inland rather than decaying geometrically after each hop.
const DEFAULT_EVAPOTRANSPIRATION_RATE: f32 = 0.0;

/// Default cap on moisture that any tile can accumulate.
///
/// `f32::MAX` (disabled) by default for backwards compatibility. Setting a
/// finite value prevents tiles from accumulating unbounded moisture over many
/// iterations; without a cap, ocean and coastal tiles dominate the moisture
/// field and make interior land values appear near-zero in visualizations.
const DEFAULT_MOISTURE_CAP: f32 = f32::MAX;

/// Default isotropic diffusion rate.
///
/// Zero by default for backwards compatibility. A value of 0.05–0.2 models
/// turbulent atmospheric mixing, distributing moisture equally in all six
/// hex directions independent of wind. This enables North-South moisture
/// transport that the purely zonal wind field cannot provide.
const DEFAULT_DIFFUSION_RATE: f32 = 0.0;

// ------------- Parameter struct -------------

/// Tuning parameters for the climate stage (design §1.10).
///
/// Controls the temperature field, wind field, evaporation, and the
/// moisture-advection / orographic-precipitation loop.
///
/// # Stability of values
///
/// Changing any default changes the generation output and breaks every
/// committed reference hash. Treat default changes the same as
/// algorithmic changes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClimateParams {
    // ------------- Step 1: temperature field -------------
    /// Normalised temperature at the equator.
    ///
    /// The equator is the row at `r = (height − 1) / 2`. Must be
    /// finite and greater than `pole_temperature`.
    /// Default: 1.0.
    pub equator_temperature: f32,

    /// Normalised temperature at the poles.
    ///
    /// Applied at `r = 0` (north pole) and `r = height − 1`
    /// (south pole). Must be finite and less than
    /// `equator_temperature`. Default: −0.5.
    pub pole_temperature: f32,

    /// Temperature drop per unit of elevation above sea level.
    ///
    /// Subtracted from the latitude-based temperature for every unit
    /// of positive elevation. Must be finite and `≥ 0.0`.
    /// Default: 0.25.
    pub elevation_lapse_rate: f32,

    /// Global temperature shift added after all other calculations.
    ///
    /// Zero at the tuning layer. The advanced-layer [`crate::ClimateBand`]
    /// maps to an offset that is added here when preparing parameters.
    /// Must be finite. Default: 0.0.
    pub climate_band_offset: f32,

    // ------------- Step 2: wind field -------------
    /// Zonal wind amplitude `A` in `u(φ) = −A·cos(3π·φ)`.
    ///
    /// Determines the peak east-west wind speed. Must be finite,
    /// `≥ 0.0`, and `≤` [`CFL_MAX_WIND_AMPLITUDE`] (= 0.25) to
    /// satisfy the CFL stability condition. Default: 0.18.
    pub wind_amplitude: f32,

    /// Amplitude of the low-frequency noise perturbation added to the
    /// zonal wind profile.
    ///
    /// Breaks up the perfectly uniform zonal pattern. Must be finite
    /// and `≥ 0.0`. Kept small to avoid CFL violations after adding
    /// to the base wind. Default: 0.04.
    pub wind_noise_amplitude: f32,

    /// Spatial frequency of the wind noise, in fractions of map width.
    ///
    /// `1.0` means the noise completes a full cycle across the map.
    /// Must be finite and `> 0.0`. Default: 0.5.
    pub wind_noise_frequency: f32,

    // ------------- Step 3: evaporation -------------
    /// Per-iteration evaporation rate for water tiles.
    ///
    /// The base evaporation added to a water tile per iteration, before
    /// temperature scaling. Must be finite and `≥ 0.0`. Default: 0.05.
    pub evaporation_rate: f32,

    /// Temperature floor below which no evaporation occurs.
    ///
    /// Water tiles whose temperature is at or below this value
    /// contribute zero evaporation. Must be finite. Default: −0.2.
    pub evaporation_temp_floor: f32,

    /// Linear scaling factor for the evaporation–temperature relationship.
    ///
    /// The effective evaporation rate is `evaporation_rate ×
    /// clamp((temp − evaporation_temp_floor) × evaporation_temp_scale,
    /// 0.0, 1.0)`. Must be finite. Default: 1.0.
    pub evaporation_temp_scale: f32,

    // ------------- Steps 4–5: advection and precipitation -------------
    /// Fraction of moisture precipitated per iteration in flat terrain.
    ///
    /// Applied before orographic and temperature factors. Must be
    /// finite and in `[0.0, 1.0]`. Default: 0.02.
    pub base_precipitation_rate: f32,

    /// Sigmoid offset for the temperature–moisture-capacity curve.
    ///
    /// The capacity factor `σ((temp − offset) × scale)` equals 0.5
    /// when `temp == temp_capacity_offset`. Must be finite.
    /// Default: 0.5.
    pub temp_capacity_offset: f32,

    /// Sigmoid steepness for the temperature–moisture-capacity curve.
    ///
    /// Higher values sharpen the transition. Must be finite and `> 0.0`.
    /// Default: 4.0.
    pub temp_capacity_scale: f32,

    /// Multiplier on positive elevation gain for orographic precipitation.
    ///
    /// When a tile is higher than its upwind neighbour, the precipitation
    /// fraction is multiplied by `1 + orographic_factor × max(0, Δelev)`.
    /// Must be finite and `≥ 0.0`. Default: 1.5.
    pub orographic_factor: f32,

    /// Hard cap on the fraction of tile moisture precipitated per iteration.
    ///
    /// Prevents numerical blow-up regardless of orographic or temperature
    /// factors. Must be finite and in `(0.0, 1.0]`. Default: 0.6.
    pub max_precipitation_fraction: f32,

    // ------------- Step 3b: coastal moisture source -------------
    /// Per-iteration moisture added to each land tile per adjacent water tile.
    ///
    /// Models sea breeze and coastal humidity independently of wind direction.
    /// A land tile with N water neighbours receives `coastal_moisture_factor × N`
    /// moisture per iteration. Must be finite and `≥ 0.0`. Default: 0.0.
    pub coastal_moisture_factor: f32,

    // ------------- Step 4b: evapotranspiration -------------
    /// Fraction of a land tile's moisture recycled back per iteration.
    ///
    /// Models plant transpiration and soil evaporation: each land tile re-emits
    /// `evapotranspiration_rate × moisture` per iteration, allowing moisture to
    /// travel further inland. Must be finite and `≥ 0.0`. Default: 0.0.
    pub evapotranspiration_rate: f32,

    // ------------- Step 3c: moisture cap -------------
    /// Maximum moisture any tile can accumulate per iteration.
    ///
    /// After each advection step, every tile is clamped to this value.
    /// Prevents ocean and coastal land tiles from dominating the moisture field.
    /// `f32::MAX` disables the cap. Must be `> 0.0` and finite (or `f32::MAX`).
    /// Default: `f32::MAX` (disabled).
    pub moisture_cap: f32,

    // ------------- Step 4c: isotropic diffusion -------------
    /// Fraction of each tile's moisture distributed isotropically per iteration.
    ///
    /// Each tile sends `diffusion_rate / 6` of its moisture to each of its six
    /// neighbours, independent of wind direction. Models turbulent atmospheric
    /// mixing and enables North-South moisture transport. Must be finite,
    /// `≥ 0.0`, and `≤ 0.5`. Default: 0.0.
    pub diffusion_rate: f32,

    // ------------- Step 6: iteration count -------------
    /// Multiplier on `map_width` to derive the iteration count.
    ///
    /// `iteration_count = max(iteration_count_min,
    /// round(iteration_count_factor × map_width))`. Must be finite
    /// and `> 0.0`. Default: 1.5.
    pub iteration_count_factor: f32,

    /// Minimum iteration count regardless of map size.
    ///
    /// Prevents very small maps from converging with too few iterations.
    /// Default: 100.
    pub iteration_count_min: u32,
}

impl ClimateParams {
    /// Returns the default climate parameters.
    ///
    /// All fields are set to the baseline values established during
    /// Stage 3.2 tuning (see
    /// `documents/crates/worldgen/parameters/climate.md`).
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            equator_temperature: DEFAULT_EQUATOR_TEMPERATURE,
            pole_temperature: DEFAULT_POLE_TEMPERATURE,
            elevation_lapse_rate: DEFAULT_ELEVATION_LAPSE_RATE,
            climate_band_offset: DEFAULT_CLIMATE_BAND_OFFSET,
            wind_amplitude: DEFAULT_WIND_AMPLITUDE,
            wind_noise_amplitude: DEFAULT_WIND_NOISE_AMPLITUDE,
            wind_noise_frequency: DEFAULT_WIND_NOISE_FREQUENCY,
            evaporation_rate: DEFAULT_EVAPORATION_RATE,
            evaporation_temp_floor: DEFAULT_EVAPORATION_TEMP_FLOOR,
            evaporation_temp_scale: DEFAULT_EVAPORATION_TEMP_SCALE,
            base_precipitation_rate: DEFAULT_BASE_PRECIPITATION_RATE,
            temp_capacity_offset: DEFAULT_TEMP_CAPACITY_OFFSET,
            temp_capacity_scale: DEFAULT_TEMP_CAPACITY_SCALE,
            orographic_factor: DEFAULT_OROGRAPHIC_FACTOR,
            max_precipitation_fraction: DEFAULT_MAX_PRECIPITATION_FRACTION,
            iteration_count_factor: DEFAULT_ITERATION_COUNT_FACTOR,
            iteration_count_min: DEFAULT_ITERATION_COUNT_MIN,
            coastal_moisture_factor: DEFAULT_COASTAL_MOISTURE_FACTOR,
            evapotranspiration_rate: DEFAULT_EVAPOTRANSPIRATION_RATE,
            moisture_cap: DEFAULT_MOISTURE_CAP,
            diffusion_rate: DEFAULT_DIFFUSION_RATE,
        }
    }

    /// Validates all field ranges.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if:
    /// - `pole_temperature >= equator_temperature`,
    /// - `elevation_lapse_rate < 0.0` or is non-finite,
    /// - `climate_band_offset` is non-finite,
    /// - `wind_amplitude > CFL_MAX_WIND_AMPLITUDE` (= 0.25) or is non-finite or `< 0.0`,
    /// - `wind_noise_amplitude < 0.0` or is non-finite,
    /// - `wind_noise_frequency <= 0.0` or is non-finite,
    /// - `evaporation_rate < 0.0` or is non-finite,
    /// - `evaporation_temp_floor` or `evaporation_temp_scale` is non-finite,
    /// - `coastal_moisture_factor < 0.0` or is non-finite,
    /// - `evapotranspiration_rate < 0.0` or is non-finite,
    /// - `moisture_cap` is `≤ 0.0` or non-finite (unless `f32::MAX`),
    /// - `diffusion_rate` is outside `[0.0, 0.5]` or non-finite,
    /// - `base_precipitation_rate` is outside `[0.0, 1.0]` or non-finite,
    /// - `temp_capacity_offset` is non-finite,
    /// - `temp_capacity_scale <= 0.0` or is non-finite,
    /// - `orographic_factor < 0.0` or is non-finite,
    /// - `max_precipitation_fraction` is outside `(0.0, 1.0]` or non-finite,
    /// - `iteration_count_factor <= 0.0` or is non-finite.
    pub fn validate(&self) -> Result<(), GenerationError> {
        if !self.equator_temperature.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "equator_temperature {} is not finite",
                    self.equator_temperature
                ),
            });
        }
        if !self.pole_temperature.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!("pole_temperature {} is not finite", self.pole_temperature),
            });
        }
        if self.pole_temperature >= self.equator_temperature {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "pole_temperature {} must be less than equator_temperature {}",
                    self.pole_temperature, self.equator_temperature
                ),
            });
        }
        if !self.elevation_lapse_rate.is_finite() || self.elevation_lapse_rate < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "elevation_lapse_rate {} must be finite and >= 0.0",
                    self.elevation_lapse_rate
                ),
            });
        }
        if !self.climate_band_offset.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "climate_band_offset {} is not finite",
                    self.climate_band_offset
                ),
            });
        }
        self.validate_wind()?;
        self.validate_evaporation()?;
        self.validate_precipitation()
    }

    fn validate_wind(&self) -> Result<(), GenerationError> {
        if !self.wind_amplitude.is_finite()
            || self.wind_amplitude < 0.0
            || self.wind_amplitude > CFL_MAX_WIND_AMPLITUDE
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "wind_amplitude {} must be finite, >= 0.0, and <= {CFL_MAX_WIND_AMPLITUDE} (CFL cap)",
                    self.wind_amplitude
                ),
            });
        }
        if !self.wind_noise_amplitude.is_finite() || self.wind_noise_amplitude < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "wind_noise_amplitude {} must be finite and >= 0.0",
                    self.wind_noise_amplitude
                ),
            });
        }
        let max_wind_magnitude =
            self.wind_amplitude + std::f32::consts::SQRT_2 * self.wind_noise_amplitude;
        if max_wind_magnitude > CFL_MAX_WIND_AMPLITUDE {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "wind_amplitude {} plus worst-case wind_noise_amplitude {} \
                     can produce magnitude {max_wind_magnitude}, exceeding CFL cap {CFL_MAX_WIND_AMPLITUDE}",
                    self.wind_amplitude, self.wind_noise_amplitude
                ),
            });
        }
        if !self.wind_noise_frequency.is_finite() || self.wind_noise_frequency <= 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "wind_noise_frequency {} must be finite and > 0.0",
                    self.wind_noise_frequency
                ),
            });
        }
        Ok(())
    }

    fn validate_evaporation(&self) -> Result<(), GenerationError> {
        if !self.evaporation_rate.is_finite() || self.evaporation_rate < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "evaporation_rate {} must be finite and >= 0.0",
                    self.evaporation_rate
                ),
            });
        }
        if !self.evaporation_temp_floor.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "evaporation_temp_floor {} is not finite",
                    self.evaporation_temp_floor
                ),
            });
        }
        if !self.evaporation_temp_scale.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "evaporation_temp_scale {} is not finite",
                    self.evaporation_temp_scale
                ),
            });
        }
        if !self.coastal_moisture_factor.is_finite() || self.coastal_moisture_factor < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "coastal_moisture_factor {} must be finite and >= 0.0",
                    self.coastal_moisture_factor
                ),
            });
        }
        if !self.evapotranspiration_rate.is_finite() || self.evapotranspiration_rate < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "evapotranspiration_rate {} must be finite and >= 0.0",
                    self.evapotranspiration_rate
                ),
            });
        }
        // f32::MAX is allowed as the "disabled" sentinel; any other value must be > 0.0 and finite.
        #[allow(clippy::float_cmp)]
        if self.moisture_cap != f32::MAX
            && (!self.moisture_cap.is_finite() || self.moisture_cap <= 0.0)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "moisture_cap {} must be > 0.0 and finite (or f32::MAX to disable)",
                    self.moisture_cap
                ),
            });
        }
        Ok(())
    }

    fn validate_precipitation(&self) -> Result<(), GenerationError> {
        if !self.diffusion_rate.is_finite()
            || self.diffusion_rate < 0.0
            || self.diffusion_rate > MAX_DIFFUSION_RATE
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "diffusion_rate {} must be finite and in [0.0, {MAX_DIFFUSION_RATE}]",
                    self.diffusion_rate
                ),
            });
        }
        if !self.base_precipitation_rate.is_finite()
            || self.base_precipitation_rate < 0.0
            || self.base_precipitation_rate > 1.0
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "base_precipitation_rate {} must be finite and in [0.0, 1.0]",
                    self.base_precipitation_rate
                ),
            });
        }
        if !self.temp_capacity_offset.is_finite() {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "temp_capacity_offset {} is not finite",
                    self.temp_capacity_offset
                ),
            });
        }
        if !self.temp_capacity_scale.is_finite() || self.temp_capacity_scale <= 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "temp_capacity_scale {} must be finite and > 0.0",
                    self.temp_capacity_scale
                ),
            });
        }
        if !self.orographic_factor.is_finite() || self.orographic_factor < 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "orographic_factor {} must be finite and >= 0.0",
                    self.orographic_factor
                ),
            });
        }
        if !self.max_precipitation_fraction.is_finite()
            || self.max_precipitation_fraction <= 0.0
            || self.max_precipitation_fraction > 1.0
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "max_precipitation_fraction {} must be finite and in (0.0, 1.0]",
                    self.max_precipitation_fraction
                ),
            });
        }
        if !self.iteration_count_factor.is_finite() || self.iteration_count_factor <= 0.0 {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "iteration_count_factor {} must be finite and > 0.0",
                    self.iteration_count_factor
                ),
            });
        }
        Ok(())
    }

    /// Derives the advection iteration count for a map of the given width.
    ///
    /// Returns `max(iteration_count_min, round(iteration_count_factor × width))`.
    #[must_use]
    pub fn iteration_count(&self, map_width: u32) -> u32 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let from_factor = (self.iteration_count_factor * map_width as f32).round() as u32;
        from_factor.max(self.iteration_count_min)
    }

    /// Returns `true` if every field equals its default value.
    ///
    /// Used by the achievement gate.
    #[must_use]
    pub fn is_all_default(&self) -> bool {
        *self == Self::defaults()
    }
}

impl Default for ClimateParams {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod defaults {
        use super::*;

        #[test]
        fn defaults_validate() {
            ClimateParams::defaults()
                .validate()
                .expect("defaults must always validate");
        }

        #[test]
        fn defaults_are_all_default() {
            assert!(ClimateParams::defaults().is_all_default());
        }

        #[test]
        fn iteration_count_is_at_least_min() {
            let p = ClimateParams::defaults();
            assert!(p.iteration_count(1) >= p.iteration_count_min);
        }

        #[test]
        fn iteration_count_scales_with_width() {
            let p = ClimateParams::defaults();
            assert!(p.iteration_count(240) > p.iteration_count(10));
        }
    }

    mod wind_amplitude {
        use super::*;

        #[test]
        fn rejects_wind_amplitude_above_cfl_cap() {
            let params = ClimateParams {
                wind_amplitude: CFL_MAX_WIND_AMPLITUDE + 0.01,
                ..ClimateParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_wind_amplitude_at_cfl_cap() {
            let params = ClimateParams {
                wind_amplitude: CFL_MAX_WIND_AMPLITUDE,
                wind_noise_amplitude: 0.0,
                ..ClimateParams::defaults()
            };
            params
                .validate()
                .expect("amplitude at CFL cap must validate");
        }

        #[test]
        fn rejects_negative_wind_amplitude() {
            let params = ClimateParams {
                wind_amplitude: -0.1,
                ..ClimateParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_wind_plus_noise_above_cfl_cap() {
            let params = ClimateParams {
                wind_amplitude: 0.24,
                wind_noise_amplitude: 0.02,
                ..ClimateParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }
    }

    mod temperature_bounds {
        use super::*;

        #[test]
        fn rejects_pole_temperature_above_equator() {
            let params = ClimateParams {
                pole_temperature: 2.0,
                equator_temperature: 1.0,
                ..ClimateParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn rejects_equal_temperatures() {
            let params = ClimateParams {
                pole_temperature: 1.0,
                equator_temperature: 1.0,
                ..ClimateParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }
    }

    mod precipitation_bounds {
        use super::*;

        #[test]
        fn rejects_zero_max_precipitation_fraction() {
            let params = ClimateParams {
                max_precipitation_fraction: 0.0,
                ..ClimateParams::defaults()
            };
            assert!(matches!(
                params.validate(),
                Err(GenerationError::InvalidParams { .. })
            ));
        }

        #[test]
        fn accepts_max_precipitation_fraction_at_one() {
            let params = ClimateParams {
                max_precipitation_fraction: 1.0,
                ..ClimateParams::defaults()
            };
            params.validate().expect("fraction = 1.0 must validate");
        }
    }
}
