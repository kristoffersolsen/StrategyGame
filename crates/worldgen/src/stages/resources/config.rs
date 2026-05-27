//! Resource placement config loading and validation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::error::GenerationError;
use crate::world_data::{Biome, MineralCharacter, ResourceId};

/// Designer-editable resource placement config.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ResourceConfig {
    /// Strategic resources with baseline plus concentration placement.
    strategic: BTreeMap<ResourceId, ResourceEntry>,
    /// Luxury resources with clustered concentration placement.
    luxury: BTreeMap<ResourceId, ResourceEntry>,
    /// Bonus resources with uniform biome/terrain scatter.
    bonus: BTreeMap<ResourceId, ResourceEntry>,
}

impl ResourceConfig {
    /// Loads a resource config from a RON file and validates it.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidResourceConfig`] if the file
    /// cannot be read, parsed, or validated.
    pub(crate) fn load(path: &Path) -> Result<Self, GenerationError> {
        let content = read_config_file(path)?;
        let config = ron::from_str::<Self>(&content).map_err(|source| {
            GenerationError::InvalidResourceConfig {
                path: path.display().to_string(),
                reason: format!("failed to parse RON: {source}"),
            }
        })?;
        config.validate(path)?;
        Ok(config)
    }

    /// Returns all entries in stable category and resource order.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (ResourceId, &ResourceEntry)> {
        self.bonus
            .iter()
            .chain(self.strategic.iter())
            .chain(self.luxury.iter())
            .map(|(&id, entry)| (id, entry))
    }

    /// Returns bonus entries in stable resource order.
    pub(crate) fn bonus_entries(&self) -> impl Iterator<Item = (ResourceId, &ResourceEntry)> {
        self.bonus.iter().map(|(&id, entry)| (id, entry))
    }

    /// Returns strategic entries in stable resource order.
    pub(crate) fn strategic_entries(&self) -> impl Iterator<Item = (ResourceId, &ResourceEntry)> {
        self.strategic.iter().map(|(&id, entry)| (id, entry))
    }

    /// Returns luxury entries in stable resource order.
    pub(crate) fn luxury_entries(&self) -> impl Iterator<Item = (ResourceId, &ResourceEntry)> {
        self.luxury.iter().map(|(&id, entry)| (id, entry))
    }

    fn validate(&self, path: &Path) -> Result<(), GenerationError> {
        let path_string = path.display().to_string();
        let mut seen = BTreeSet::new();

        validate_section(
            &self.strategic,
            ResourceCategory::Strategic,
            &path_string,
            &mut seen,
        )?;
        validate_section(
            &self.luxury,
            ResourceCategory::Luxury,
            &path_string,
            &mut seen,
        )?;
        validate_section(
            &self.bonus,
            ResourceCategory::Bonus,
            &path_string,
            &mut seen,
        )?;

        for &resource in ResourceId::ALL {
            if !seen.contains(&resource) {
                return Err(GenerationError::InvalidResourceConfig {
                    path: path_string,
                    reason: format!("missing entry for {resource:?}"),
                });
            }
        }
        Ok(())
    }
}

fn validate_section(
    entries: &BTreeMap<ResourceId, ResourceEntry>,
    expected: ResourceCategory,
    path: &str,
    seen: &mut BTreeSet<ResourceId>,
) -> Result<(), GenerationError> {
    for (&resource, entry) in entries {
        if !seen.insert(resource) {
            return Err(GenerationError::InvalidResourceConfig {
                path: path.to_owned(),
                reason: format!("duplicate entry for {resource:?}"),
            });
        }
        if entry.category != expected {
            return Err(GenerationError::InvalidResourceConfig {
                path: path.to_owned(),
                reason: format!(
                    "{resource:?} appears in {expected:?} section but has category {:?}",
                    entry.category
                ),
            });
        }
        entry.validate(resource, path)?;
    }
    Ok(())
}

fn read_config_file(path: &Path) -> Result<String, GenerationError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(first_error) if path.is_relative() => {
            let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path);
            std::fs::read_to_string(&workspace_path).map_err(|second_error| {
                GenerationError::InvalidResourceConfig {
                    path: path.display().to_string(),
                    reason: format!(
                        "failed to read file from current directory ({first_error}) \
                         or workspace root candidate {} ({second_error})",
                        workspace_path.display()
                    ),
                }
            })
        }
        Err(source) => Err(GenerationError::InvalidResourceConfig {
            path: path.display().to_string(),
            reason: format!("failed to read file: {source}"),
        }),
    }
}

/// Resource placement category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ResourceCategory {
    /// Mixed baseline plus plate-weighted scatter.
    Strategic,
    /// Clustered plate-weighted deposits.
    Luxury,
    /// Uniform biome/terrain scatter.
    Bonus,
}

/// One resource's placement parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ResourceEntry {
    /// Placement category.
    pub(crate) category: ResourceCategory,
    /// Eligible biome list. Empty means all biome-assigned land tiles.
    #[serde(default)]
    pub(crate) eligible_biomes: Vec<Biome>,
    /// Terrain constraints. Empty means no additional terrain constraint.
    #[serde(default)]
    pub(crate) eligible_terrain: Vec<TerrainFilter>,
    /// Optional normalized latitude band.
    pub(crate) latitude_preference: Option<LatitudePreference>,
    /// Target fraction of eligible tiles.
    pub(crate) rarity: f32,
    /// Whether this resource uses cluster expansion.
    pub(crate) cluster: bool,
    /// Neighbor radius used for cluster expansion.
    pub(crate) cluster_radius: u8,
    /// Minimum distance between separate deposits.
    pub(crate) min_spacing: u8,
    /// Fraction placed through the uniform baseline pass.
    pub(crate) baseline_fraction: f32,
    /// Mineral-character affinity weights.
    #[serde(default)]
    pub(crate) mineral_affinities: BTreeMap<MineralCharacter, f32>,
}

impl ResourceEntry {
    fn validate(&self, resource: ResourceId, path: &str) -> Result<(), GenerationError> {
        if !self.rarity.is_finite() || !(0.0..=1.0).contains(&self.rarity) || self.rarity == 0.0 {
            return Err(invalid_entry(
                path,
                resource,
                format!("rarity {} must be finite and in (0.0, 1.0]", self.rarity),
            ));
        }
        if self.min_spacing == 0 {
            return Err(invalid_entry(
                path,
                resource,
                "min_spacing must be at least 1".to_owned(),
            ));
        }
        if self.cluster && self.cluster_radius == 0 {
            return Err(invalid_entry(
                path,
                resource,
                "cluster_radius must be at least 1 when cluster = true".to_owned(),
            ));
        }
        if matches!(
            self.category,
            ResourceCategory::Strategic | ResourceCategory::Luxury
        ) && (!self.baseline_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.baseline_fraction))
        {
            return Err(invalid_entry(
                path,
                resource,
                format!(
                    "baseline_fraction {} must be finite and in [0.0, 1.0]",
                    self.baseline_fraction
                ),
            ));
        }
        for (&mineral, &weight) in &self.mineral_affinities {
            if !weight.is_finite() || weight < 0.0 {
                return Err(invalid_entry(
                    path,
                    resource,
                    format!("affinity for {mineral:?} must be finite and non-negative"),
                ));
            }
        }
        if let Some(latitude) = self.latitude_preference {
            latitude.validate(resource, path)?;
        }
        Ok(())
    }

    /// Returns `true` if any configured terrain filter targets water.
    pub(crate) fn targets_water(&self) -> bool {
        self.eligible_terrain
            .iter()
            .any(|&filter| filter.targets_water())
    }
}

fn invalid_entry(
    path: &str,
    resource: ResourceId,
    reason: impl std::fmt::Display,
) -> GenerationError {
    GenerationError::InvalidResourceConfig {
        path: path.to_owned(),
        reason: format!("{resource:?}: {reason}"),
    }
}

/// Terrain or hydrology filter for resource eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum TerrainFilter {
    /// Any non-water, non-mountain biome tile.
    Land,
    /// Any ocean, coast, or lake tile.
    Water,
    /// Deep ocean height level.
    Ocean,
    /// Coast height level.
    Coast,
    /// Inland lake tile.
    Lake,
    /// Non-hilly land tile.
    FlatLand,
    /// Hilly land tile.
    Hills,
    /// Mountain height level.
    Mountain,
}

impl TerrainFilter {
    fn targets_water(self) -> bool {
        matches!(self, Self::Water | Self::Ocean | Self::Coast | Self::Lake)
    }
}

/// Normalized latitude band, where `0.0` is equatorial and `1.0` is polar.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) struct LatitudePreference {
    /// Minimum normalized latitude.
    pub(crate) min: f32,
    /// Maximum normalized latitude.
    pub(crate) max: f32,
}

impl LatitudePreference {
    fn validate(self, resource: ResourceId, path: &str) -> Result<(), GenerationError> {
        if !self.min.is_finite()
            || !self.max.is_finite()
            || !(0.0..=1.0).contains(&self.min)
            || !(0.0..=1.0).contains(&self.max)
            || self.min > self.max
        {
            return Err(invalid_entry(
                path,
                resource,
                format!(
                    "latitude_preference ({}, {}) must be finite, ordered, and in [0.0, 1.0]",
                    self.min, self.max
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads() {
        let path = Path::new("assets/worldgen/resources.ron");
        ResourceConfig::load(path).expect("default resource config must validate");
    }
}
