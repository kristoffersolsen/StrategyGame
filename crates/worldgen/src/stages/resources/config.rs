//! Resource placement config loading and validation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::error::GenerationError;
use crate::world_data::{
    BiomeHandle, BiomeRegistry, MineralCharacter, ResourceCategory, ResourceHandle,
    ResourceRegistry,
};

// ------------- Raw (serde) types for RON deserialization -------------

/// Raw RON form of the resource config, keyed by string IDs.
#[derive(Debug, Clone, serde::Deserialize)]
struct RawResourceConfig {
    /// Strategic resources with baseline plus concentration placement.
    strategic: BTreeMap<String, RawResourceEntry>,
    /// Luxury resources with clustered concentration placement.
    luxury: BTreeMap<String, RawResourceEntry>,
    /// Bonus resources with uniform biome/terrain scatter.
    bonus: BTreeMap<String, RawResourceEntry>,
}

/// Raw form of one resource entry, using string biome IDs.
#[derive(Debug, Clone, serde::Deserialize)]
struct RawResourceEntry {
    /// Placement category (must match the section it appears in).
    category: ResourceCategory,
    /// Eligible biome string IDs. Empty means all biome-assigned land tiles.
    #[serde(default)]
    eligible_biomes: Vec<String>,
    /// Terrain constraints. Empty means no additional terrain constraint.
    #[serde(default)]
    eligible_terrain: Vec<TerrainFilter>,
    /// Optional normalized latitude band.
    latitude_preference: Option<LatitudePreference>,
    /// Target fraction of eligible tiles.
    rarity: f32,
    /// Whether this resource uses cluster expansion.
    cluster: bool,
    /// Neighbor radius used for cluster expansion.
    cluster_radius: u8,
    /// Minimum distance between separate deposits.
    min_spacing: u8,
    /// Fraction placed through the uniform baseline pass.
    baseline_fraction: f32,
    /// Mineral-character affinity weights.
    #[serde(default)]
    mineral_affinities: BTreeMap<MineralCharacter, f32>,
}

// ------------- Resolved (handle-keyed) config types -------------

/// Designer-editable resource placement config.
pub(crate) struct ResourceConfig {
    /// Strategic resources with baseline plus concentration placement.
    strategic: BTreeMap<ResourceHandle, ResourceEntry>,
    /// Luxury resources with clustered concentration placement.
    luxury: BTreeMap<ResourceHandle, ResourceEntry>,
    /// Bonus resources with uniform biome/terrain scatter.
    bonus: BTreeMap<ResourceHandle, ResourceEntry>,
    /// Registry built from the loaded resource names.
    resource_registry: ResourceRegistry,
}

impl ResourceConfig {
    /// Loads a resource config from a RON file and validates it.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidResourceConfig`] if the file
    /// cannot be read, parsed, or validated.
    pub(crate) fn load(
        path: &Path,
        biome_registry: &BiomeRegistry,
    ) -> Result<Self, GenerationError> {
        let content = read_config_file(path)?;
        let raw = ron::from_str::<RawResourceConfig>(&content).map_err(|source| {
            GenerationError::InvalidResourceConfig {
                path: path.display().to_string(),
                reason: format!("failed to parse RON: {source}"),
            }
        })?;
        Self::from_raw(&raw, path, biome_registry)
    }

    fn from_raw(
        raw: &RawResourceConfig,
        path: &Path,
        biome_registry: &BiomeRegistry,
    ) -> Result<Self, GenerationError> {
        let path_str = path.display().to_string();

        // Collect all resource IDs across all sections to build registry.
        // Check for duplicates across sections.
        let mut all_ids: BTreeSet<String> = BTreeSet::new();
        for id in raw
            .strategic
            .keys()
            .chain(raw.luxury.keys())
            .chain(raw.bonus.keys())
        {
            if !all_ids.insert(id.clone()) {
                return Err(GenerationError::InvalidResourceConfig {
                    path: path_str,
                    reason: format!("duplicate resource entry for {id:?}"),
                });
            }
        }

        // Build the resource registry from all collected IDs.
        let registry_inner = content::Registry::build(all_ids.iter().map(String::as_str));
        let mut category_vec = vec![ResourceCategory::Bonus; registry_inner.len()];
        for id in raw.strategic.keys() {
            let h = registry_inner.resolve(id).expect("id came from build");
            category_vec[h.index()] = ResourceCategory::Strategic;
        }
        for id in raw.luxury.keys() {
            let h = registry_inner.resolve(id).expect("id came from build");
            category_vec[h.index()] = ResourceCategory::Luxury;
        }
        // bonus section keeps the default ResourceCategory::Bonus

        let resource_registry = ResourceRegistry {
            inner: registry_inner,
            category: category_vec,
        };

        // Resolve each section.
        let strategic = resolve_section(
            &raw.strategic,
            ResourceCategory::Strategic,
            &resource_registry,
            biome_registry,
            &path_str,
        )?;
        let luxury = resolve_section(
            &raw.luxury,
            ResourceCategory::Luxury,
            &resource_registry,
            biome_registry,
            &path_str,
        )?;
        let bonus = resolve_section(
            &raw.bonus,
            ResourceCategory::Bonus,
            &resource_registry,
            biome_registry,
            &path_str,
        )?;

        Ok(Self {
            strategic,
            luxury,
            bonus,
            resource_registry,
        })
    }

    /// Returns all entries in stable category and resource order.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (ResourceHandle, &ResourceEntry)> {
        self.bonus
            .iter()
            .chain(self.strategic.iter())
            .chain(self.luxury.iter())
            .map(|(&id, entry)| (id, entry))
    }

    /// Returns bonus entries in stable resource order.
    pub(crate) fn bonus_entries(&self) -> impl Iterator<Item = (ResourceHandle, &ResourceEntry)> {
        self.bonus.iter().map(|(&id, entry)| (id, entry))
    }

    /// Returns strategic entries in stable resource order.
    pub(crate) fn strategic_entries(
        &self,
    ) -> impl Iterator<Item = (ResourceHandle, &ResourceEntry)> {
        self.strategic.iter().map(|(&id, entry)| (id, entry))
    }

    /// Returns luxury entries in stable resource order.
    pub(crate) fn luxury_entries(&self) -> impl Iterator<Item = (ResourceHandle, &ResourceEntry)> {
        self.luxury.iter().map(|(&id, entry)| (id, entry))
    }

    /// Returns the resource registry built from this config.
    pub(crate) fn resource_registry(&self) -> &ResourceRegistry {
        &self.resource_registry
    }
}

fn resolve_section(
    raw_section: &BTreeMap<String, RawResourceEntry>,
    expected_category: ResourceCategory,
    resource_registry: &ResourceRegistry,
    biome_registry: &BiomeRegistry,
    path_str: &str,
) -> Result<BTreeMap<ResourceHandle, ResourceEntry>, GenerationError> {
    let mut resolved = BTreeMap::new();
    for (id, raw_entry) in raw_section {
        if raw_entry.category != expected_category {
            return Err(GenerationError::InvalidResourceConfig {
                path: path_str.to_owned(),
                reason: format!(
                    "{id:?} appears in {expected_category:?} section but has category {:?}",
                    raw_entry.category
                ),
            });
        }

        // Resolve eligible biome strings to handles.
        let mut eligible_biomes = Vec::with_capacity(raw_entry.eligible_biomes.len());
        for biome_id in &raw_entry.eligible_biomes {
            let handle = biome_registry.resolve(biome_id).ok_or_else(|| {
                GenerationError::InvalidResourceConfig {
                    path: path_str.to_owned(),
                    reason: format!(
                        "{id:?}: eligible biome {biome_id:?} not found in biome registry"
                    ),
                }
            })?;
            eligible_biomes.push(handle);
        }

        let entry = ResourceEntry {
            category: raw_entry.category,
            eligible_biomes,
            eligible_terrain: raw_entry.eligible_terrain.clone(),
            latitude_preference: raw_entry.latitude_preference,
            rarity: raw_entry.rarity,
            cluster: raw_entry.cluster,
            cluster_radius: raw_entry.cluster_radius,
            min_spacing: raw_entry.min_spacing,
            baseline_fraction: raw_entry.baseline_fraction,
            mineral_affinities: raw_entry.mineral_affinities.clone(),
        };

        validate_entry(&entry, id, path_str)?;
        let handle = resource_registry
            .resolve(id)
            .expect("id came from registry build");
        resolved.insert(handle, entry);
    }
    Ok(resolved)
}

fn validate_entry(entry: &ResourceEntry, id: &str, path: &str) -> Result<(), GenerationError> {
    if !entry.rarity.is_finite() || !(0.0..=1.0).contains(&entry.rarity) || entry.rarity == 0.0 {
        return Err(invalid_entry(
            path,
            id,
            format!("rarity {} must be finite and in (0.0, 1.0]", entry.rarity),
        ));
    }
    if entry.min_spacing == 0 {
        return Err(invalid_entry(
            path,
            id,
            "min_spacing must be at least 1".to_owned(),
        ));
    }
    if entry.cluster && entry.cluster_radius == 0 {
        return Err(invalid_entry(
            path,
            id,
            "cluster_radius must be at least 1 when cluster = true".to_owned(),
        ));
    }
    if matches!(
        entry.category,
        ResourceCategory::Strategic | ResourceCategory::Luxury
    ) && (!entry.baseline_fraction.is_finite()
        || !(0.0..=1.0).contains(&entry.baseline_fraction))
    {
        return Err(invalid_entry(
            path,
            id,
            format!(
                "baseline_fraction {} must be finite and in [0.0, 1.0]",
                entry.baseline_fraction
            ),
        ));
    }
    for (&mineral, &weight) in &entry.mineral_affinities {
        if !weight.is_finite() || weight < 0.0 {
            return Err(invalid_entry(
                path,
                id,
                format!("affinity for {mineral:?} must be finite and non-negative"),
            ));
        }
    }
    if let Some(latitude) = entry.latitude_preference {
        validate_latitude(latitude, id, path)?;
    }
    Ok(())
}

fn validate_latitude(
    latitude: LatitudePreference,
    id: &str,
    path: &str,
) -> Result<(), GenerationError> {
    if !latitude.min.is_finite()
        || !latitude.max.is_finite()
        || !(0.0..=1.0).contains(&latitude.min)
        || !(0.0..=1.0).contains(&latitude.max)
        || latitude.min > latitude.max
    {
        return Err(invalid_entry(
            path,
            id,
            format!(
                "latitude_preference ({}, {}) must be finite, ordered, and in [0.0, 1.0]",
                latitude.min, latitude.max
            ),
        ));
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

fn invalid_entry(path: &str, id: &str, reason: impl std::fmt::Display) -> GenerationError {
    GenerationError::InvalidResourceConfig {
        path: path.to_owned(),
        reason: format!("{id:?}: {reason}"),
    }
}

/// One resource's placement parameters.
pub(crate) struct ResourceEntry {
    /// Placement category.
    pub(crate) category: ResourceCategory,
    /// Eligible biome list (resolved handles). Empty means all biome-assigned land tiles.
    pub(crate) eligible_biomes: Vec<BiomeHandle>,
    /// Terrain constraints. Empty means no additional terrain constraint.
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
    pub(crate) mineral_affinities: BTreeMap<MineralCharacter, f32>,
}

impl ResourceEntry {
    /// Returns `true` if any configured terrain filter targets water.
    pub(crate) fn targets_water(&self) -> bool {
        self.eligible_terrain
            .iter()
            .any(|&filter| filter.targets_water())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_biome_registry() -> BiomeRegistry {
        let inner = content::Registry::build([
            "boreal_forest",
            "desert",
            "grassland",
            "temperate_forest",
            "tropical_rainforest",
            "tundra",
        ]);
        let n = inner.len();
        BiomeRegistry {
            is_forest: vec![false; n],
            inner,
        }
    }

    #[test]
    fn default_config_loads() {
        let path = Path::new("assets/worldgen/resources.ron");
        let biome_registry = test_biome_registry();
        ResourceConfig::load(path, &biome_registry).expect("default resource config must validate");
    }
}
