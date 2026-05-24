//! Generation input parameters.
//!
//! Three nested layers matching the player-exposure tiers from design
//! doc §1.15:
//!
//! - [`BasicParams`] — flavor choices (map archetype). Any value
//!   produces a playable map.
//! - [`AdvancedParams`] — strategic environment (size, climate band,
//!   resource abundance, ...). Hard-capped at 6–8 sliders.
//! - [`TuningParams`] — every internal generation knob. Extreme values
//!   may produce broken maps; intended for experimentation.
//!
//! The split is a contract with the future UI: each layer maps to a
//! distinct menu, and the achievement gate fires on whether any
//! [`TuningParams`] field differs from its default.
//!
//! All variants of every enum here have explicit `Default` impls so
//! that an unconfigured [`WorldParams`] produces a sensible, playable
//! medium-sized continents map.

use crate::error::GenerationError;

/// The complete set of generation parameters and the master seed.
///
///
/// `WorldParams` is `PartialEq` but not `Eq` because [`AdvancedParams`]
/// holds an `f32` (`land_fraction`). Determinism does not require `Eq`
/// — generation hashes the bit pattern via the determinism scaffold
/// (see Stage 2.4).
#[derive(Debug, Clone, PartialEq)]
pub struct WorldParams {
    /// Master RNG seed. Every stage derives a sub-RNG from this value.
    pub seed: u64,
    /// Flavor layer — map archetype.
    pub basic: BasicParams,
    /// Strategic layer — size, climate, abundance, etc.
    pub advanced: AdvancedParams,
    /// Full tunability layer — every internal generation knob.
    pub tuning: TuningParams,
}

impl WorldParams {
    /// Returns parameters describing a default medium continents map.
    ///
    /// Equivalent to [`Self::default`] but `const`, so it can be used
    /// in contexts that require compile-time evaluation.
    #[must_use]
    pub const fn const_default() -> Self {
        Self {
            seed: 0,
            basic: BasicParams::const_default(),
            advanced: AdvancedParams::const_default(),
            tuning: TuningParams::const_default(),
        }
    }

    /// Validates the parameter set for internal consistency.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if any field is
    /// outside the range its type was declared to permit. This is a
    /// defence-in-depth check; the enum-typed fields make most
    /// invalid configurations unrepresentable at compile time.
    pub fn validate(&self) -> Result<(), GenerationError> {
        let (width, height) = self.basic.map_size.dimensions();
        if width == 0 || height == 0 {
            return Err(GenerationError::InvalidParams {
                reason: format!("map dimensions must be non-zero, got {width}x{height}"),
            });
        }
        if self.advanced.player_count == 0 {
            return Err(GenerationError::InvalidParams {
                reason: "player_count must be at least 1".to_owned(),
            });
        }
        Ok(())
    }
}

impl Default for WorldParams {
    fn default() -> Self {
        Self::const_default()
    }
}

// ------------- Basic layer -------------

/// Flavor layer parameters. See [`MapArchetype`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BasicParams {
    /// High-level map shape archetype.
    pub map_size: MapSize,
    /// High-level continental arrangement.
    pub archetype: MapArchetype,
}

impl BasicParams {
    /// `const` form of [`Self::default`].
    #[must_use]
    pub const fn const_default() -> Self {
        Self {
            map_size: MapSize::Medium,
            archetype: MapArchetype::Continents,
        }
    }
}

impl Default for BasicParams {
    fn default() -> Self {
        Self::const_default()
    }
}

/// High-level continental arrangement. Design doc §1.15 Basic layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapArchetype {
    /// Single supercontinent with peripheral islands.
    Pangaea,
    /// Several medium continents separated by ocean.
    Continents,
    /// Many small landmasses with significant ocean.
    Archipelago,
    /// Mostly ocean with scattered small islands.
    Islands,
}

// ------------- Advanced layer -------------

/// Strategic layer parameters. Sliders the player adjusts to choose a
/// playable strategic environment.
///
/// Per design doc §1.15, this layer is hard-capped at 6–8 fields.
/// Currently 6:
/// 1. `map_size` (in [`BasicParams`] for ergonomic reasons)
/// 2. Land/water ratio
/// 3. Climate band
/// 4. World age
/// 5. Resource abundance
/// 6. Player count
///
/// Map shape modifier and river density are deferred until their
/// generation stages exist to consume them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdvancedParams {
    /// Target fraction of tiles that are land. Range `[0.0, 1.0]`,
    /// recommended `0.3..=0.6`.
    pub land_fraction: f32,
    /// Global temperature shift.
    pub climate_band: ClimateBand,
    /// Affects mountain prevalence and erosion smoothness.
    pub world_age: WorldAge,
    /// Density of placed luxury and strategic resources.
    pub resource_abundance: ResourceAbundance,
    /// Number of players to seat. Drives starting-position selection.
    pub player_count: u32,
}

impl AdvancedParams {
    /// `const` form of [`Self::default`].
    #[must_use]
    pub const fn const_default() -> Self {
        Self {
            land_fraction: 0.4,
            climate_band: ClimateBand::Temperate,
            world_age: WorldAge::Standard,
            resource_abundance: ResourceAbundance::Standard,
            player_count: 4,
        }
    }
}

impl Default for AdvancedParams {
    fn default() -> Self {
        Self::const_default()
    }
}

// We intentionally do not derive `Eq`/`Hash` for `AdvancedParams`:
// `land_fraction` is `f32`, so the type cannot be `Eq`. Determinism is
// guaranteed by validating the bit pattern in tests; equality
// comparisons in code should use field-wise tolerant comparison.

/// Map size archetype. The named variants pick concrete `(width,
/// height)` pairs designed to keep tile counts predictable. Custom
/// dimensions are available via [`Self::Custom`].
///
/// Default is [`Self::Medium`] (240×150 ≈ 36k tiles), the working
/// assumption from design doc §1.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapSize {
    /// 120×80, ≈ 9.6k tiles.
    Small,
    /// 180×120, ≈ 21.6k tiles.
    Standard,
    /// 240×150, ≈ 36k tiles. Default.
    Medium,
    /// 320×200, ≈ 64k tiles.
    Large,
    /// 400×250, ≈ 100k tiles.
    Huge,
    /// Caller-specified dimensions. Both must be non-zero.
    Custom {
        /// Wrap-axis (east-west) extent.
        width: u32,
        /// Bounded-axis (north-south) extent.
        height: u32,
    },
}

impl MapSize {
    /// Returns the concrete `(width, height)` for this size.
    ///
    /// For non-`Custom` variants the dimensions are fixed; for
    /// [`Self::Custom`] the caller-supplied values are returned
    /// directly. Validation that the values are non-zero happens in
    /// [`WorldParams::validate`].
    #[must_use]
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Small => (120, 80),
            Self::Standard => (180, 120),
            Self::Medium => (240, 150),
            Self::Large => (320, 200),
            Self::Huge => (400, 250),
            Self::Custom { width, height } => (width, height),
        }
    }
}

/// Global temperature shift applied during climate generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClimateBand {
    /// Lower global temperature; more tundra and boreal.
    Cold,
    /// Default; full range of biomes.
    #[default]
    Temperate,
    /// Higher global temperature; more desert and tropical.
    Hot,
}

/// World age affects mountain prevalence and erosion smoothness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldAge {
    /// Sharp, recently uplifted mountains; less erosion.
    Young,
    /// Balanced default.
    #[default]
    Standard,
    /// Worn-down topography; few mountains, deep valleys.
    Old,
}

/// Density of placed luxury and strategic resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ResourceAbundance {
    /// Roughly 60% of standard placement count.
    Scarce,
    /// Default.
    #[default]
    Standard,
    /// Roughly 150% of standard placement count.
    Abundant,
}

// ------------- Tuning layer -------------

/// Full-tunability layer parameters.
///
/// Every internal generation knob lives here. Fields are added by each
/// pipeline stage as it lands; Stage 2 ships this type as an empty
/// struct so the layer split is in place from the start.
///
/// Per design doc §1.15, setting any field to a non-default value
/// disables achievements for the resulting game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct TuningParams {
    // No fields yet. Tectonics will add plate-count, plate-jitter,
    // mountain-falloff, etc. Climate will add advection-iterations,
    // wind-speed, orographic-factor, evaporation-rate. And so on.
}

impl TuningParams {
    /// `const` form of [`Self::default`].
    #[must_use]
    pub const fn const_default() -> Self {
        Self {}
    }

    /// Returns `true` if every field equals its default.
    ///
    /// Used by the achievement gate: only when this returns `true` for
    /// the [`TuningParams`] of an active game are achievements
    /// awarded.
    #[must_use]
    pub fn is_all_default(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params_pass_validation() {
        WorldParams::default()
            .validate()
            .expect("default params must always validate");
    }

    #[test]
    fn custom_zero_width_fails_validation() {
        let params = WorldParams {
            basic: BasicParams {
                map_size: MapSize::Custom {
                    width: 0,
                    height: 100,
                },
                ..BasicParams::default()
            },
            ..WorldParams::default()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }

    #[test]
    fn zero_player_count_fails_validation() {
        let params = WorldParams {
            advanced: AdvancedParams {
                player_count: 0,
                ..AdvancedParams::default()
            },
            ..WorldParams::default()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }

    #[test]
    fn fresh_tuning_params_are_all_default() {
        assert!(TuningParams::default().is_all_default());
    }

    #[test]
    fn map_size_default_is_medium() {
        let (w, h) = BasicParams::default().map_size.dimensions();
        assert_eq!((w, h), (240, 150));
    }
}
