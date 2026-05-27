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
use crate::world_data::{Biome, BiomeData, ClimateData, HydrologyData};

/// Designer-editable Whittaker lookup table for biome classification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct BiomeTable {
    /// Number of temperature bins covered by `entries`.
    temperature_bin_count: u8,
    /// Number of moisture bins covered by `entries`.
    moisture_bin_count: u8,
    /// Flat list of table entries covering every `(t, m)` pair once.
    entries: Vec<BiomeTableEntry>,
    /// Biome-to-biome reclassification map for hydrology overrides.
    pub(super) hydrology_overrides: BTreeMap<Biome, Biome>,
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
        let table = ron::from_str::<Self>(&content).map_err(|source| {
            GenerationError::InvalidBiomeTable {
                path: path.display().to_string(),
                reason: format!("failed to parse RON: {source}"),
            }
        })?;
        table.validate(path, params)?;
        Ok(table)
    }

    /// Validates grid dimensions, full coverage, and duplicate entries.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidBiomeTable`] if the table does
    /// not exactly cover the configured grid.
    pub(crate) fn validate(
        &self,
        path: &Path,
        params: &BiomeParams,
    ) -> Result<(), GenerationError> {
        let path = path.display().to_string();
        if self.temperature_bin_count != params.temperature_bin_count {
            return Err(GenerationError::InvalidBiomeTable {
                path,
                reason: format!(
                    "temperature_bin_count {} does not match params {}",
                    self.temperature_bin_count, params.temperature_bin_count
                ),
            });
        }
        if self.moisture_bin_count != params.moisture_bin_count {
            return Err(GenerationError::InvalidBiomeTable {
                path,
                reason: format!(
                    "moisture_bin_count {} does not match params {}",
                    self.moisture_bin_count, params.moisture_bin_count
                ),
            });
        }

        let mut seen = BTreeSet::new();
        for entry in &self.entries {
            if entry.temperature_bin >= self.temperature_bin_count {
                return Err(GenerationError::InvalidBiomeTable {
                    path,
                    reason: format!(
                        "temperature bin {} is outside 0..{}",
                        entry.temperature_bin, self.temperature_bin_count
                    ),
                });
            }
            if entry.moisture_bin >= self.moisture_bin_count {
                return Err(GenerationError::InvalidBiomeTable {
                    path,
                    reason: format!(
                        "moisture bin {} is outside 0..{}",
                        entry.moisture_bin, self.moisture_bin_count
                    ),
                });
            }
            let key = (entry.temperature_bin, entry.moisture_bin);
            if !seen.insert(key) {
                return Err(GenerationError::InvalidBiomeTable {
                    path,
                    reason: format!(
                        "duplicate entry for temperature {}, moisture {}",
                        key.0, key.1
                    ),
                });
            }
        }

        for temperature_bin in 0..self.temperature_bin_count {
            for moisture_bin in 0..self.moisture_bin_count {
                if !seen.contains(&(temperature_bin, moisture_bin)) {
                    return Err(GenerationError::InvalidBiomeTable {
                        path,
                        reason: format!(
                            "missing entry for temperature {temperature_bin}, moisture {moisture_bin}"
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    fn lookup(&self, temperature_bin: u8, moisture_bin: u8) -> Biome {
        self.entries
            .iter()
            .find(|entry| {
                entry.temperature_bin == temperature_bin && entry.moisture_bin == moisture_bin
            })
            .map(|entry| entry.biome)
            .expect("BiomeTable::validate guarantees full table coverage")
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

/// One cell in the biome lookup table.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct BiomeTableEntry {
    /// Temperature bin index.
    #[serde(rename = "t")]
    temperature_bin: u8,
    /// Moisture bin index.
    #[serde(rename = "m")]
    moisture_bin: u8,
    /// Biome assigned to this `(t, m)` pair.
    biome: Biome,
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
    let forest_density =
        forest_density::build_forest_density(bounds, &biome, params, forest_noise_seed);

    BiomeData {
        biome,
        forest_density,
    }
}
