//! Tuning parameters for starting position selection (Stage 3.6, design §1.14).

use std::collections::BTreeMap;

use crate::error::GenerationError;

const DEFAULT_NEIGHBORHOOD_RADIUS: u32 = 2;
const DEFAULT_ADEQUACY_FLOOR: f32 = 10.0;
const DEFAULT_MIN_SPACING: u32 = 8;
const DEFAULT_FRESHWATER_FOOD_BONUS: f32 = 1.0;
const DEFAULT_HILLY_PRODUCTION_BONUS: f32 = 1.0;
const DEFAULT_FOOD_WEIGHT: f32 = 0.6;
const DEFAULT_PRODUCTION_WEIGHT: f32 = 0.4;

const MIN_NEIGHBORHOOD_RADIUS: u32 = 1;
const MAX_NEIGHBORHOOD_RADIUS: u32 = 5;
const MIN_ADEQUACY_FLOOR: f32 = 0.0;
const MAX_ADEQUACY_FLOOR: f32 = 200.0;
const MIN_MIN_SPACING: u32 = 2;
const MAX_MIN_SPACING: u32 = 30;

/// Food yield proxy for each biome, keyed by stable biome string ID.
fn default_biome_food_scores() -> BTreeMap<String, f32> {
    [
        ("boreal_forest", 1.0_f32),
        ("desert", 0.0),
        ("grassland", 2.5),
        ("temperate_forest", 2.0),
        ("tropical_rainforest", 3.0),
        ("tundra", 0.5),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

/// Production yield proxy for each biome, keyed by stable biome string ID.
fn default_biome_production_scores() -> BTreeMap<String, f32> {
    [
        ("boreal_forest", 1.5_f32),
        ("desert", 0.5),
        ("grassland", 1.0),
        ("temperate_forest", 1.5),
        ("tropical_rainforest", 0.5),
        ("tundra", 0.5),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

/// Tuning parameters for starting position selection (Stage 3.6, design §1.14).
///
/// The viability heuristic sums per-tile yield proxies over a
/// neighbourhood of `neighborhood_radius` hex steps. A tile is eligible
/// only if its neighbourhood sum reaches `adequacy_floor`. Selected
/// positions must be at least `min_spacing` hex steps apart.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StartingPositionParams {
    /// Neighbourhood radius (hex steps) for summing yield scores.
    ///
    /// Valid range: `1..=5`. Default: 2 (centre + 18 tiles).
    pub neighborhood_radius: u32,

    /// Minimum neighbourhood yield sum for a tile to be eligible.
    ///
    /// Tiles below this value are excluded from candidacy.
    /// Valid range: `0.0..=200.0`. Default: 10.0.
    pub adequacy_floor: f32,

    /// Minimum hex distance between any two selected positions.
    ///
    /// Valid range: `2..=30`. Default: 8.
    pub min_spacing: u32,

    /// Food yield proxy per biome string ID.
    ///
    /// Keys are stable biome string IDs (e.g. `"grassland"`). Missing
    /// biomes default to `0.0`. Resolved to a dense handle-indexed
    /// array once at stage startup.
    #[serde(default = "default_biome_food_scores")]
    pub biome_food_scores: BTreeMap<String, f32>,

    /// Production yield proxy per biome string ID.
    ///
    /// Same keying and resolution as [`Self::biome_food_scores`].
    #[serde(default = "default_biome_production_scores")]
    pub biome_production_scores: BTreeMap<String, f32>,

    /// Additional food yield applied to floodplain and river tiles.
    ///
    /// Default: 1.0.
    pub freshwater_food_bonus: f32,

    /// Additional production yield applied to hilly tiles.
    ///
    /// Default: 1.0.
    pub hilly_production_bonus: f32,

    /// Weight of the food component in the combined viability score.
    ///
    /// Default: 0.6.
    pub food_weight: f32,

    /// Weight of the production component in the combined viability score.
    ///
    /// Default: 0.4.
    pub production_weight: f32,
}

impl StartingPositionParams {
    /// Returns the default starting position parameters.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            neighborhood_radius: DEFAULT_NEIGHBORHOOD_RADIUS,
            adequacy_floor: DEFAULT_ADEQUACY_FLOOR,
            min_spacing: DEFAULT_MIN_SPACING,
            biome_food_scores: default_biome_food_scores(),
            biome_production_scores: default_biome_production_scores(),
            freshwater_food_bonus: DEFAULT_FRESHWATER_FOOD_BONUS,
            hilly_production_bonus: DEFAULT_HILLY_PRODUCTION_BONUS,
            food_weight: DEFAULT_FOOD_WEIGHT,
            production_weight: DEFAULT_PRODUCTION_WEIGHT,
        }
    }

    /// Validates all field ranges.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if any field falls
    /// outside its documented range.
    pub fn validate(&self) -> Result<(), GenerationError> {
        if !(MIN_NEIGHBORHOOD_RADIUS..=MAX_NEIGHBORHOOD_RADIUS).contains(&self.neighborhood_radius)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "neighborhood_radius {} must be in \
                     {MIN_NEIGHBORHOOD_RADIUS}..={MAX_NEIGHBORHOOD_RADIUS}",
                    self.neighborhood_radius
                ),
            });
        }
        if !(MIN_ADEQUACY_FLOOR..=MAX_ADEQUACY_FLOOR).contains(&self.adequacy_floor) {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "adequacy_floor {} must be in \
                     {MIN_ADEQUACY_FLOOR}..={MAX_ADEQUACY_FLOOR}",
                    self.adequacy_floor
                ),
            });
        }
        if !(MIN_MIN_SPACING..=MAX_MIN_SPACING).contains(&self.min_spacing) {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "min_spacing {} must be in \
                     {MIN_MIN_SPACING}..={MAX_MIN_SPACING}",
                    self.min_spacing
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

impl Default for StartingPositionParams {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        StartingPositionParams::defaults()
            .validate()
            .expect("defaults must always validate");
    }

    #[test]
    fn fresh_params_are_all_default() {
        assert!(StartingPositionParams::defaults().is_all_default());
    }

    #[test]
    fn invalid_neighborhood_radius_fails() {
        let params = StartingPositionParams {
            neighborhood_radius: 0,
            ..StartingPositionParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }

    #[test]
    fn invalid_min_spacing_fails() {
        let params = StartingPositionParams {
            min_spacing: 1,
            ..StartingPositionParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }
}
