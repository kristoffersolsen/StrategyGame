//! Biome assignment (Stage 3.4, design §1.12).
//!
//! Classifies land tiles through a designer-editable Whittaker lookup
//! table, applies hydrology-driven fertile overrides, optionally smooths
//! isolated one-tile artifacts, and emits a forest-density feature field.

mod forest_density;
mod hydrology_overrides;
mod lookup;
mod smoothing;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use coords::MapBounds;
use rand_core::Rng;

use crate::error::GenerationError;
use crate::params::BiomeParams;
use crate::rng::StageRng;
use crate::world_data::{BiomeData, BiomeHandle, BiomeRegistry, ClimateData, HydrologyData};

// ------------- Raw (serde) types for RON deserialization -------------

/// Raw RON entry for one biome descriptor.
#[derive(Debug, Clone, serde::Deserialize)]
struct RawBiomeDescriptor {
    /// Whether this biome is a forest biome.
    is_forest: bool,
}

/// Raw RON entry for one biome table cell.
#[derive(Debug, Clone, serde::Deserialize)]
struct RawBiomeTableEntry {
    /// Temperature bin index.
    #[serde(rename = "t")]
    temperature_bin: u8,
    /// Moisture bin index.
    #[serde(rename = "m")]
    moisture_bin: u8,
    /// String biome ID.
    biome: String,
}

/// Raw form of the biome table as deserialized from RON.
#[derive(Debug, Clone, serde::Deserialize)]
struct RawBiomeTable {
    /// Grid dimensions.
    temperature_bin_count: u8,
    /// Grid dimensions.
    moisture_bin_count: u8,
    /// All `(t, m, biome_id)` entries.
    entries: Vec<RawBiomeTableEntry>,
    /// Hydrology overrides: source biome id → replacement biome id.
    hydrology_overrides: BTreeMap<String, String>,
    /// Biome descriptors keyed by string id.
    biomes: BTreeMap<String, RawBiomeDescriptor>,
}

// ------------- Resolved (handle-keyed) BiomeTable -------------

/// Designer-editable Whittaker lookup table for biome classification.
///
/// The raw RON form uses string IDs; this resolved form uses dense
/// handles for all hot-path lookups. Built by [`BiomeTable::load`].
pub(crate) struct BiomeTable {
    /// Number of moisture bins (used for flat array indexing).
    moisture_bin_count: u8,
    /// Flat array indexed by `t * moisture_bin_count + m`.
    lookup: Vec<BiomeHandle>,
    /// Hydrology overrides: `hydrology_overrides[handle.index()]` is
    /// `Some(replacement)` when the source biome has an override.
    pub(super) hydrology_overrides: Vec<Option<BiomeHandle>>,
    /// Registry built from the `biomes` section of the RON file.
    registry: BiomeRegistry,
}

impl BiomeTable {
    /// Loads a biome table from a RON file and validates it against `params`.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidBiomeTable`] if the file cannot
    /// be read, parsed, or validated.
    pub(crate) fn load(path: &Path, params: &BiomeParams) -> Result<Self, GenerationError> {
        let content = read_table_file(path)?;
        let raw = ron::from_str::<RawBiomeTable>(&content).map_err(|source| {
            GenerationError::InvalidBiomeTable {
                path: path.display().to_string(),
                reason: format!("failed to parse RON: {source}"),
            }
        })?;
        Self::from_raw(&raw, path, params)
    }

    #[allow(clippy::too_many_lines)]
    fn from_raw(
        raw: &RawBiomeTable,
        path: &Path,
        params: &BiomeParams,
    ) -> Result<Self, GenerationError> {
        let path_str = path.display().to_string();

        // Build registry from biomes section.
        let biome_ids: Vec<&str> = raw.biomes.keys().map(String::as_str).collect();
        let registry_inner = content::Registry::build(biome_ids);
        let mut is_forest = vec![false; registry_inner.len()];
        for (id, desc) in &raw.biomes {
            let handle = registry_inner
                .resolve(id)
                .expect("id came from registry build, must resolve");
            is_forest[handle.index()] = desc.is_forest;
        }
        let registry = BiomeRegistry {
            inner: registry_inner,
            is_forest,
        };

        // Validate grid dimensions.
        if raw.temperature_bin_count != params.temperature_bin_count {
            return Err(GenerationError::InvalidBiomeTable {
                path: path_str,
                reason: format!(
                    "temperature_bin_count {} does not match params {}",
                    raw.temperature_bin_count, params.temperature_bin_count
                ),
            });
        }
        if raw.moisture_bin_count != params.moisture_bin_count {
            return Err(GenerationError::InvalidBiomeTable {
                path: path_str,
                reason: format!(
                    "moisture_bin_count {} does not match params {}",
                    raw.moisture_bin_count, params.moisture_bin_count
                ),
            });
        }

        // Build flat lookup array.
        let cell_count =
            usize::from(raw.temperature_bin_count) * usize::from(raw.moisture_bin_count);
        let mut lookup = vec![content::Handle::from_raw(0); cell_count];
        let mut seen = BTreeSet::new();
        for entry in &raw.entries {
            if entry.temperature_bin >= raw.temperature_bin_count {
                return Err(GenerationError::InvalidBiomeTable {
                    path: path_str,
                    reason: format!(
                        "temperature bin {} is outside 0..{}",
                        entry.temperature_bin, raw.temperature_bin_count
                    ),
                });
            }
            if entry.moisture_bin >= raw.moisture_bin_count {
                return Err(GenerationError::InvalidBiomeTable {
                    path: path_str,
                    reason: format!(
                        "moisture bin {} is outside 0..{}",
                        entry.moisture_bin, raw.moisture_bin_count
                    ),
                });
            }
            let key = (entry.temperature_bin, entry.moisture_bin);
            if !seen.insert(key) {
                return Err(GenerationError::InvalidBiomeTable {
                    path: path_str,
                    reason: format!(
                        "duplicate entry for temperature {}, moisture {}",
                        key.0, key.1
                    ),
                });
            }
            let handle = registry.resolve(&entry.biome).ok_or_else(|| {
                GenerationError::InvalidBiomeTable {
                    path: path_str.clone(),
                    reason: format!("entry biome {:?} is not in the biomes section", entry.biome),
                }
            })?;
            let idx = usize::from(entry.temperature_bin) * usize::from(raw.moisture_bin_count)
                + usize::from(entry.moisture_bin);
            lookup[idx] = handle;
        }

        // Validate full coverage.
        for temperature_bin in 0..raw.temperature_bin_count {
            for moisture_bin in 0..raw.moisture_bin_count {
                if !seen.contains(&(temperature_bin, moisture_bin)) {
                    return Err(GenerationError::InvalidBiomeTable {
                        path: path_str,
                        reason: format!(
                            "missing entry for temperature {temperature_bin}, moisture {moisture_bin}"
                        ),
                    });
                }
            }
        }

        // Build hydrology overrides as dense handle-indexed Vec.
        let n = registry.inner.len();
        let mut hydrology_overrides = vec![None; n];
        for (source_id, target_id) in &raw.hydrology_overrides {
            let source =
                registry
                    .resolve(source_id)
                    .ok_or_else(|| GenerationError::InvalidBiomeTable {
                        path: path_str.clone(),
                        reason: format!(
                            "hydrology_override source {source_id:?} is not in the biomes section"
                        ),
                    })?;
            let target =
                registry
                    .resolve(target_id)
                    .ok_or_else(|| GenerationError::InvalidBiomeTable {
                        path: path_str.clone(),
                        reason: format!(
                            "hydrology_override target {target_id:?} is not in the biomes section"
                        ),
                    })?;
            hydrology_overrides[source.index()] = Some(target);
        }

        Ok(Self {
            moisture_bin_count: raw.moisture_bin_count,
            lookup,
            hydrology_overrides,
            registry,
        })
    }

    /// Returns the biome handle for the given `(temperature_bin, moisture_bin)` pair.
    pub(crate) fn lookup(&self, temperature_bin: u8, moisture_bin: u8) -> BiomeHandle {
        let idx = usize::from(temperature_bin) * usize::from(self.moisture_bin_count)
            + usize::from(moisture_bin);
        self.lookup[idx]
    }

    /// Returns a reference to the biome registry.
    pub(crate) fn registry(&self) -> &BiomeRegistry {
        &self.registry
    }
}

fn read_table_file(path: &Path) -> Result<String, GenerationError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(first_error) if path.is_relative() => {
            let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path);
            std::fs::read_to_string(&workspace_path).map_err(|second_error| {
                GenerationError::InvalidBiomeTable {
                    path: path.display().to_string(),
                    reason: format!(
                        "failed to read file from current directory ({first_error}) \
                         or workspace root candidate {} ({second_error})",
                        workspace_path.display()
                    ),
                }
            })
        }
        Err(source) => Err(GenerationError::InvalidBiomeTable {
            path: path.display().to_string(),
            reason: format!("failed to read file: {source}"),
        }),
    }
}

/// Runs the biome stage end-to-end and returns the populated [`BiomeData`].
pub(crate) fn run(
    bounds: MapBounds,
    climate: &ClimateData,
    hydrology: &HydrologyData,
    params: &BiomeParams,
    mountain_level: u8,
    table: &BiomeTable,
    rng: &mut StageRng,
) -> BiomeData {
    let mut biome =
        lookup::assign_base_biomes(bounds, climate, hydrology, params, mountain_level, table);
    hydrology_overrides::apply_hydrology_overrides(bounds, hydrology, params, table, &mut biome);
    biome = smoothing::smooth_biomes(bounds, &biome, hydrology, params);

    let forest_noise_seed = rng.next_u32();
    let forest_density = forest_density::build_forest_density(
        bounds,
        &biome,
        table.registry(),
        params,
        forest_noise_seed,
    );

    BiomeData {
        biome,
        forest_density,
    }
}
