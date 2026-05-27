//! Eligible tile filtering for resource placement.

use std::collections::BTreeMap;

use coords::{Coord, MapBounds};

use crate::world_data::{BiomeData, HydrologyData, ResourceId, TectonicData};

use super::config::{ResourceConfig, ResourceEntry, TerrainFilter};

/// Per-resource eligible tile lists.
pub(super) type Eligibility = BTreeMap<ResourceId, Vec<Coord>>;

/// Builds eligible tile lists for every configured resource.
pub(super) fn build_eligibility(
    bounds: MapBounds,
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    biomes: &BiomeData,
    config: &ResourceConfig,
) -> Eligibility {
    let mut eligibility = BTreeMap::new();
    for (resource, entry) in config.entries() {
        let tiles = hydrology
            .height_level
            .iter_coords()
            .map(|(coord, _)| coord)
            .filter(|&coord| is_eligible(coord, bounds, tectonics, hydrology, biomes, entry))
            .collect();
        eligibility.insert(resource, tiles);
    }
    eligibility
}

fn is_eligible(
    coord: Coord,
    bounds: MapBounds,
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    biomes: &BiomeData,
    entry: &ResourceEntry,
) -> bool {
    if !biome_matches(coord, biomes, entry) {
        return false;
    }
    if !terrain_matches(coord, tectonics, hydrology, biomes, entry) {
        return false;
    }
    if let Some(latitude) = entry.latitude_preference {
        let value = normalized_latitude(coord, bounds);
        if value < latitude.min || value > latitude.max {
            return false;
        }
    }
    true
}

fn biome_matches(coord: Coord, biomes: &BiomeData, entry: &ResourceEntry) -> bool {
    if entry.eligible_biomes.is_empty() {
        return entry.targets_water() || biomes.biome[coord].is_some();
    }
    let Some(tile_biome) = biomes.biome[coord] else {
        return false;
    };
    entry.eligible_biomes.contains(&tile_biome)
}

fn terrain_matches(
    coord: Coord,
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    biomes: &BiomeData,
    entry: &ResourceEntry,
) -> bool {
    if entry.eligible_terrain.is_empty() {
        return true;
    }
    entry
        .eligible_terrain
        .iter()
        .any(|&filter| terrain_filter_matches(coord, tectonics, hydrology, biomes, filter))
}

fn terrain_filter_matches(
    coord: Coord,
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    biomes: &BiomeData,
    filter: TerrainFilter,
) -> bool {
    match filter {
        TerrainFilter::Land => biomes.biome[coord].is_some(),
        TerrainFilter::Water => hydrology.height_level[coord] <= 1 || hydrology.is_lake[coord],
        TerrainFilter::Ocean => hydrology.height_level[coord] == 0 && !hydrology.is_lake[coord],
        TerrainFilter::Coast => hydrology.height_level[coord] == 1 && !hydrology.is_lake[coord],
        TerrainFilter::Lake => hydrology.is_lake[coord],
        TerrainFilter::FlatLand => biomes.biome[coord].is_some() && !tectonics.is_hilly[coord],
        TerrainFilter::Hills => biomes.biome[coord].is_some() && tectonics.is_hilly[coord],
        TerrainFilter::Mountain => {
            biomes.biome[coord].is_none() && hydrology.height_level[coord] > 1
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn normalized_latitude(coord: Coord, bounds: MapBounds) -> f32 {
    let row_center = coord.r as f32 + 0.5;
    let north_to_south = row_center / bounds.height() as f32;
    ((north_to_south - 0.5).abs() * 2.0).clamp(0.0, 1.0)
}
