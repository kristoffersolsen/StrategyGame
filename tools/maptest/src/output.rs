//! Image writing helpers shared by the `run` and `sweep` subcommands.
//!
//! `write_world_outputs` writes all debug images for a single world
//! into a structured output directory. The caller supplies a `label`
//! string that becomes the image filename stem, allowing `run` to
//! use `seed_N` and `sweep` to use `v<value>_seed<N>`.
//!
//! Call `ensure_output_dirs` once per base directory (from the main
//! thread, before spawning rayon tasks) to avoid race conditions when
//! multiple workers write to the same set of subdirectories.

use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use worldgen::{
    BiomeData, BoundaryType, ClimateData, Direction, Field, HydrologyData, ResourceData, RiverSize,
    TectonicData, WorldData,
    debug_viz::{
        BiomeCmap, BoundaryCmap, Categorical, Colormap, Grayscale, HeightLevelCmap, ResourceCmap,
        Viridis, biome_handle_to_f32, dump_field_hex, dump_field_pixels,
        dump_field_with_seed_overlay, dump_rgb_field_hex, resource_handle_to_f32,
    },
};

// ------------- Subdirectory names (stable across run and sweep) -------------

pub(crate) const SUBDIR_SEEDS: &str = "seeds";
pub(crate) const SUBDIR_PLATE_HEX: &str = "plate_map_hex";
pub(crate) const SUBDIR_BOUNDARY: &str = "boundary_types";
pub(crate) const SUBDIR_ELEVATION: &str = "elevation";
pub(crate) const SUBDIR_LEVELS_HEX: &str = "height_levels_hex";
pub(crate) const SUBDIR_HILLS: &str = "hills";
pub(crate) const SUBDIR_TEMPERATURE_HEX: &str = "temperature_hex";
pub(crate) const SUBDIR_WIND: &str = "wind";
pub(crate) const SUBDIR_WIND_NS: &str = "wind_ns";
pub(crate) const SUBDIR_MOISTURE: &str = "moisture";
pub(crate) const SUBDIR_PRECIPITATION_HEX: &str = "precipitation_hex";
pub(crate) const SUBDIR_PRECIPITATION_LOG: &str = "precipitation_log";
pub(crate) const SUBDIR_PRECIPITATION_CSV: &str = "precipitation_csv";
pub(crate) const SUBDIR_FLOW_DIRECTION: &str = "flow_direction";
pub(crate) const SUBDIR_FLOW_ACCUMULATION: &str = "flow_accumulation";
pub(crate) const SUBDIR_LAKES: &str = "lakes";
pub(crate) const SUBDIR_RIVERS: &str = "rivers";
pub(crate) const SUBDIR_FLOODPLAINS: &str = "floodplains";
pub(crate) const SUBDIR_CANYONS: &str = "canyons";
pub(crate) const SUBDIR_HYDROLOGY_OVERLAY: &str = "hydrology_overlay";
pub(crate) const SUBDIR_BIOMES: &str = "biomes";
pub(crate) const SUBDIR_FOREST_DENSITY: &str = "forest_density";
pub(crate) const SUBDIR_RESOURCES: &str = "resources";
pub(crate) const SUBDIR_RESOURCE_IRON: &str = "resource_iron";
pub(crate) const SUBDIR_RESOURCE_HORSES: &str = "resource_horses";
pub(crate) const SUBDIR_RESOURCE_GOLD: &str = "resource_gold";
pub(crate) const SUBDIR_RESOURCE_GEMS: &str = "resource_gems";
pub(crate) const SUBDIR_RESOURCE_SPICES: &str = "resource_spices";
pub(crate) const SUBDIR_RESOURCE_WINE: &str = "resource_wine";
pub(crate) const SUBDIR_RESOURCE_WHEAT: &str = "resource_wheat";
pub(crate) const SUBDIR_RESOURCE_FISH: &str = "resource_fish";
pub(crate) const SUBDIR_RESOURCE_STONE: &str = "resource_stone";

pub(crate) const ALL_SUBDIRS: &[&str] = &[
    SUBDIR_SEEDS,
    SUBDIR_PLATE_HEX,
    SUBDIR_BOUNDARY,
    SUBDIR_ELEVATION,
    SUBDIR_LEVELS_HEX,
    SUBDIR_HILLS,
    SUBDIR_TEMPERATURE_HEX,
    SUBDIR_WIND,
    SUBDIR_WIND_NS,
    SUBDIR_MOISTURE,
    SUBDIR_PRECIPITATION_HEX,
    SUBDIR_PRECIPITATION_LOG,
    SUBDIR_PRECIPITATION_CSV,
    SUBDIR_FLOW_DIRECTION,
    SUBDIR_FLOW_ACCUMULATION,
    SUBDIR_LAKES,
    SUBDIR_RIVERS,
    SUBDIR_FLOODPLAINS,
    SUBDIR_CANYONS,
    SUBDIR_HYDROLOGY_OVERLAY,
    SUBDIR_BIOMES,
    SUBDIR_FOREST_DENSITY,
    SUBDIR_RESOURCES,
    SUBDIR_RESOURCE_IRON,
    SUBDIR_RESOURCE_HORSES,
    SUBDIR_RESOURCE_GOLD,
    SUBDIR_RESOURCE_GEMS,
    SUBDIR_RESOURCE_SPICES,
    SUBDIR_RESOURCE_WINE,
    SUBDIR_RESOURCE_WHEAT,
    SUBDIR_RESOURCE_FISH,
    SUBDIR_RESOURCE_STONE,
];

// ------------- Directory preparation -------------

/// Creates all output subdirectories under `base_dir`.
///
/// Call this once from the main thread before running parallel tasks.
///
/// # Errors
///
/// Returns an error if any subdirectory cannot be created.
pub(crate) fn ensure_output_dirs(base_dir: &Path) -> Result<()> {
    for &name in ALL_SUBDIRS {
        let dir = base_dir.join(name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create output dir: {}", dir.display()))?;
    }
    Ok(())
}

// ------------- Image writing -------------

/// Writes all Stage 3.1 and Stage 3.2 debug images into `base_dir`.
///
/// Each image is written to `<base_dir>/<subdir>/<label>.png`.
/// Subdirectories must already exist — call [`ensure_output_dirs`] first.
///
/// `label` is the filename stem, e.g. `"seed_1"` or `"v0.4000_seed2"`.
///
/// # Errors
///
/// Returns an error if any image cannot be written to disk.
pub(crate) fn write_world_outputs(
    world: &WorldData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    land_level_count: u8,
) -> Result<()> {
    let tectonics = &world.tectonics;
    let climate = &world.climate;
    let hydrology = &world.hydrology;
    let biomes = &world.biomes;
    let resources = &world.resources;

    write_seeds(tectonics, base_dir, label)?;
    write_plate_map(tectonics, base_dir, label, hex_size)?;
    write_boundary_types(tectonics, base_dir, label)?;
    write_elevation(tectonics, base_dir, label)?;
    write_height_levels(tectonics, base_dir, label, hex_size, land_level_count)?;
    write_hills(tectonics, base_dir, label)?;
    write_temperature(climate, base_dir, label, hex_size)?;
    write_wind(climate, base_dir, label)?;
    write_wind_ns(climate, base_dir, label)?;
    write_moisture(climate, base_dir, label)?;
    write_precipitation(climate, base_dir, label, hex_size)?;
    write_precipitation_log(climate, base_dir, label, hex_size)?;
    write_precipitation_csv(climate, base_dir, label)?;
    write_flow_direction(hydrology, base_dir, label, hex_size)?;
    write_flow_accumulation(hydrology, base_dir, label, hex_size)?;
    write_lakes(hydrology, base_dir, label, hex_size, land_level_count)?;
    write_rivers(hydrology, base_dir, label, hex_size)?;
    write_floodplains(hydrology, base_dir, label)?;
    write_canyons(hydrology, base_dir, label)?;
    write_hydrology_overlay(
        tectonics,
        hydrology,
        base_dir,
        label,
        hex_size,
        land_level_count,
    )?;
    write_biomes(biomes, base_dir, label, hex_size, world)?;
    write_forest_density(biomes, base_dir, label)?;
    write_resources(resources, base_dir, label, hex_size, world)?;
    log_climate_stats(tectonics, climate);
    log_hydrology_stats(hydrology);
    log_biome_stats(biomes, world);
    log_resource_stats(resources, world);
    Ok(())
}

// ------------- Per-output helpers (tectonics) -------------

fn write_seeds(tectonics: &TectonicData, base_dir: &Path, label: &str) -> Result<()> {
    let path = base_dir.join(SUBDIR_SEEDS).join(format!("{label}.png"));
    let black = Field::filled(tectonics.plate_id.bounds(), 0.0_f32);
    dump_field_with_seed_overlay(
        &black,
        &tectonics.seeds,
        &path,
        |v| *v,
        &Grayscale,
        [255, 255, 255],
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_plate_map(
    tectonics: &TectonicData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let path = base_dir.join(SUBDIR_PLATE_HEX).join(format!("{label}.png"));
    dump_field_hex(
        &tectonics.plate_id,
        &path,
        f32::from,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_boundary_types(tectonics: &TectonicData, base_dir: &Path, label: &str) -> Result<()> {
    let path = base_dir.join(SUBDIR_BOUNDARY).join(format!("{label}.png"));
    dump_field_pixels(
        &tectonics.boundary_type,
        &path,
        |v| match v {
            None => 0.0,
            Some(BoundaryType::Transform) => 1.0,
            Some(BoundaryType::Divergent) => 2.0,
            Some(BoundaryType::Convergent) => 3.0,
        },
        &BoundaryCmap,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_elevation(tectonics: &TectonicData, base_dir: &Path, label: &str) -> Result<()> {
    let elev_min = tectonics
        .elevation
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let elev_max = tectonics
        .elevation
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let elev_range = elev_max - elev_min;

    let path = base_dir.join(SUBDIR_ELEVATION).join(format!("{label}.png"));
    dump_field_pixels(
        &tectonics.elevation,
        &path,
        |v| {
            if elev_range > 0.0 {
                (v - elev_min) / elev_range
            } else {
                0.5
            }
        },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_height_levels(
    tectonics: &TectonicData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    land_level_count: u8,
) -> Result<()> {
    let path = base_dir
        .join(SUBDIR_LEVELS_HEX)
        .join(format!("{label}.png"));
    dump_field_hex(
        &tectonics.height_level,
        &path,
        f32::from,
        &HeightLevelCmap { land_level_count },
        hex_size,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_hills(tectonics: &TectonicData, base_dir: &Path, label: &str) -> Result<()> {
    let path = base_dir.join(SUBDIR_HILLS).join(format!("{label}.png"));
    dump_field_pixels(
        &tectonics.is_hilly,
        &path,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

// ------------- Per-output helpers (climate) -------------

fn write_temperature(
    climate: &ClimateData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let t_min = climate
        .temperature
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let t_max = climate
        .temperature
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let t_range = t_max - t_min;

    let to_f32 = move |v: f32| {
        if t_range > 0.0 {
            (v - t_min) / t_range
        } else {
            0.5
        }
    };

    let path = base_dir
        .join(SUBDIR_TEMPERATURE_HEX)
        .join(format!("{label}.png"));
    dump_field_hex(&climate.temperature, &path, to_f32, &Viridis, hex_size)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn write_wind(climate: &ClimateData, base_dir: &Path, label: &str) -> Result<()> {
    let max_mag = climate
        .wind
        .iter()
        .map(|[wx, wy]| (wx * wx + wy * wy).sqrt())
        .fold(0.0_f32, f32::max);

    let path = base_dir.join(SUBDIR_WIND).join(format!("{label}.png"));
    dump_field_pixels(
        &climate.wind,
        &path,
        move |[wx, wy]: [f32; 2]| {
            if max_mag > 0.0 {
                (wx * wx + wy * wy).sqrt() / max_mag
            } else {
                0.0
            }
        },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_moisture(climate: &ClimateData, base_dir: &Path, label: &str) -> Result<()> {
    let max_moist = climate.moisture.iter().copied().fold(0.0_f32, f32::max);

    let path = base_dir.join(SUBDIR_MOISTURE).join(format!("{label}.png"));
    dump_field_pixels(
        &climate.moisture,
        &path,
        move |v| {
            if max_moist > 0.0 { v / max_moist } else { 0.0 }
        },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_precipitation(
    climate: &ClimateData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let max_precip = climate
        .precipitation
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);

    let to_f32 = move |v: f32| {
        if max_precip > 0.0 {
            v / max_precip
        } else {
            0.0
        }
    };

    let path = base_dir
        .join(SUBDIR_PRECIPITATION_HEX)
        .join(format!("{label}.png"));
    dump_field_hex(&climate.precipitation, &path, to_f32, &Viridis, hex_size)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn write_precipitation_log(
    climate: &ClimateData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let max_log = climate
        .precipitation
        .iter()
        .copied()
        .map(|v| (v + 1.0).ln())
        .fold(0.0_f32, f32::max);

    let to_f32 = move |v: f32| {
        if max_log > 0.0 {
            (v + 1.0).ln() / max_log
        } else {
            0.0
        }
    };

    let path = base_dir
        .join(SUBDIR_PRECIPITATION_LOG)
        .join(format!("{label}.png"));
    dump_field_hex(&climate.precipitation, &path, to_f32, &Viridis, hex_size)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn write_precipitation_csv(climate: &ClimateData, base_dir: &Path, label: &str) -> Result<()> {
    let w = climate.precipitation.bounds().width() as usize;
    let vals: Vec<f32> = climate.precipitation.iter().copied().collect();
    let content = vals
        .chunks(w)
        .map(|row| {
            row.iter()
                .map(|v| format!("{v:.4}"))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let path = base_dir
        .join(SUBDIR_PRECIPITATION_CSV)
        .join(format!("{label}.csv"));
    std::fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn write_wind_ns(climate: &ClimateData, base_dir: &Path, label: &str) -> Result<()> {
    // Show the north-south wind component centred on grey (0 = mid-grey, south = bright,
    // north = dark). Unlike the magnitude image this reveals directional noise.
    let max_ns = climate
        .wind
        .iter()
        .map(|v| v[1].abs())
        .fold(0.0_f32, f32::max);

    let path = base_dir.join(SUBDIR_WIND_NS).join(format!("{label}.png"));
    dump_field_pixels(
        &climate.wind,
        &path,
        move |[_wx, wy]: [f32; 2]| {
            if max_ns > 0.0 {
                wy / max_ns * 0.5 + 0.5
            } else {
                0.5
            }
        },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

// ------------- Per-output helpers (hydrology) -------------

fn write_flow_direction(
    hydrology: &HydrologyData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let path = base_dir
        .join(SUBDIR_FLOW_DIRECTION)
        .join(format!("{label}.png"));
    dump_field_hex(
        &hydrology.flow_direction,
        &path,
        flow_direction_to_f32,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_flow_accumulation(
    hydrology: &HydrologyData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let max_log = hydrology
        .flow_accumulation
        .iter()
        .copied()
        .map(|v| (v + 1.0).ln())
        .fold(0.0_f32, f32::max);
    let to_f32 = move |v: f32| {
        if max_log > 0.0 {
            (v + 1.0).ln() / max_log
        } else {
            0.0
        }
    };

    let path = base_dir
        .join(SUBDIR_FLOW_ACCUMULATION)
        .join(format!("{label}.png"));
    dump_field_hex(
        &hydrology.flow_accumulation,
        &path,
        to_f32,
        &Viridis,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_lakes(
    hydrology: &HydrologyData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    land_level_count: u8,
) -> Result<()> {
    let path = base_dir.join(SUBDIR_LAKES).join(format!("{label}.png"));
    dump_field_hex(
        &hydrology.height_level,
        &path,
        |level| {
            if level == 1 { 0.0 } else { f32::from(level) }
        },
        &HeightLevelCmap { land_level_count },
        hex_size,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_rivers(
    hydrology: &HydrologyData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    let path = base_dir.join(SUBDIR_RIVERS).join(format!("{label}.png"));
    dump_field_hex(
        &hydrology.river_size,
        &path,
        river_size_to_f32,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_floodplains(hydrology: &HydrologyData, base_dir: &Path, label: &str) -> Result<()> {
    let path = base_dir
        .join(SUBDIR_FLOODPLAINS)
        .join(format!("{label}.png"));
    dump_field_pixels(
        &hydrology.is_floodplain,
        &path,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

fn write_canyons(hydrology: &HydrologyData, base_dir: &Path, label: &str) -> Result<()> {
    let canyon_overlay = Field::from_fn(hydrology.is_canyon.bounds(), |coord| {
        if hydrology.is_canyon[coord] {
            2_u8
        } else {
            u8::from(hydrology.is_canyon_wall[coord])
        }
    });
    let path = base_dir.join(SUBDIR_CANYONS).join(format!("{label}.png"));
    dump_field_pixels(&canyon_overlay, &path, f32::from, &Categorical)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn write_hydrology_overlay(
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    land_level_count: u8,
) -> Result<()> {
    let overlay = hydrology_overlay_field(tectonics, hydrology, land_level_count);
    let path = base_dir
        .join(SUBDIR_HYDROLOGY_OVERLAY)
        .join(format!("{label}.png"));
    dump_rgb_field_hex(&overlay, &path, hex_size)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn hydrology_overlay_field(
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    land_level_count: u8,
) -> Field<[u8; 3]> {
    let height_cmap = HeightLevelCmap { land_level_count };
    Field::from_fn(tectonics.height_level.bounds(), |coord| {
        if hydrology.is_lake[coord] {
            LAKE_COLOR
        } else if hydrology.is_canyon[coord] || hydrology.is_canyon_wall[coord] {
            CANYON_COLOR
        } else if hydrology.river_size[coord] == Some(RiverSize::MajorRiver) {
            MAJOR_RIVER_COLOR
        } else if hydrology.river_size[coord].is_some() {
            RIVER_COLOR
        } else {
            height_cmap.color(f32::from(hydrology.height_level[coord]))
        }
    })
}

fn flow_direction_to_f32(direction: Option<Direction>) -> f32 {
    match direction {
        None => 0.0,
        Some(Direction::East) => 1.0,
        Some(Direction::SouthEast) => 2.0,
        Some(Direction::SouthWest) => 3.0,
        Some(Direction::West) => 4.0,
        Some(Direction::NorthWest) => 5.0,
        Some(Direction::NorthEast) => 6.0,
    }
}

fn river_size_to_f32(river_size: Option<RiverSize>) -> f32 {
    match river_size {
        None => 0.0,
        Some(RiverSize::Creek) => 1.0,
        Some(RiverSize::River) => 2.0,
        Some(RiverSize::MajorRiver) => 3.0,
    }
}

// ------------- Per-output helpers (biomes) -------------

fn write_biomes(
    biomes: &BiomeData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    world: &WorldData,
) -> Result<()> {
    let path = base_dir.join(SUBDIR_BIOMES).join(format!("{label}.png"));
    let cmap = BiomeCmap::new(&world.biome_registry);
    dump_field_hex(&biomes.biome, &path, biome_handle_to_f32, &cmap, hex_size)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn write_forest_density(biomes: &BiomeData, base_dir: &Path, label: &str) -> Result<()> {
    let path = base_dir
        .join(SUBDIR_FOREST_DENSITY)
        .join(format!("{label}.png"));
    dump_field_pixels(&biomes.forest_density, &path, |v| v, &Viridis)
        .with_context(|| format!("failed to write {}", path.display()))
}

// ------------- Per-output helpers (resources) -------------

fn write_resources(
    resources: &ResourceData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    world: &WorldData,
) -> Result<()> {
    let combined_path = base_dir.join(SUBDIR_RESOURCES).join(format!("{label}.png"));
    let cmap = ResourceCmap::new(&world.resource_registry);
    dump_field_hex(
        &resources.resource,
        &combined_path,
        resource_handle_to_f32,
        &cmap,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", combined_path.display()))?;

    for (handle, id) in world.resource_registry.inner.iter() {
        let subdir = base_dir.join(resource_subdir_for_id(id));
        let path = subdir.join(format!("{label}.png"));
        dump_field_pixels(
            &resources.resource,
            &path,
            |tile| {
                if tile == Some(handle) { 1.0 } else { 0.0 }
            },
            &Grayscale,
        )
        .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn resource_subdir_for_id(id: &str) -> &str {
    // The string ID is the subdirectory name — stable across renames.
    match id {
        "iron" => SUBDIR_RESOURCE_IRON,
        "horses" => SUBDIR_RESOURCE_HORSES,
        "gold" => SUBDIR_RESOURCE_GOLD,
        "gems" => SUBDIR_RESOURCE_GEMS,
        "spices" => SUBDIR_RESOURCE_SPICES,
        "wine" => SUBDIR_RESOURCE_WINE,
        "wheat" => SUBDIR_RESOURCE_WHEAT,
        "fish" => SUBDIR_RESOURCE_FISH,
        "stone" => SUBDIR_RESOURCE_STONE,
        _ => SUBDIR_RESOURCES, // fallback for unknown resources
    }
}

const LAKE_COLOR: [u8; 3] = [0, 170, 170];
const RIVER_COLOR: [u8; 3] = [255, 105, 180];
const MAJOR_RIVER_COLOR: [u8; 3] = [130, 70, 210];
const CANYON_COLOR: [u8; 3] = [140, 85, 35];

// Tile counts fit comfortably in f32 (max ~2 million for a huge map).
#[allow(clippy::cast_precision_loss)]
fn log_climate_stats(tectonics: &TectonicData, climate: &ClimateData) {
    let mut land_precip_sum = 0.0_f32;
    let mut water_precip_sum = 0.0_f32;
    let mut land_count = 0_usize;
    let mut water_count = 0_usize;
    let mut land_moisture_sum = 0.0_f32;

    for ((h, p), m) in tectonics
        .height_level
        .iter()
        .zip(climate.precipitation.iter())
        .zip(climate.moisture.iter())
    {
        if *h > 1 {
            land_precip_sum += *p;
            land_count += 1;
            land_moisture_sum += *m;
        } else {
            water_precip_sum += *p;
            water_count += 1;
        }
    }

    let total_precip = land_precip_sum + water_precip_sum;
    let precip_land_frac = if total_precip > 0.0 {
        land_precip_sum / total_precip
    } else {
        0.0
    };
    let precip_land_mean = if land_count > 0 {
        land_precip_sum / land_count as f32
    } else {
        0.0
    };
    let precip_water_mean = if water_count > 0 {
        water_precip_sum / water_count as f32
    } else {
        0.0
    };
    let precip_land_water_ratio = if precip_water_mean > 0.0 {
        precip_land_mean / precip_water_mean
    } else {
        0.0
    };
    let moisture_land_mean = if land_count > 0 {
        land_moisture_sum / land_count as f32
    } else {
        0.0
    };
    let tile_count = land_count + water_count;
    let wind_ns_rms = if tile_count > 0 {
        let sum_sq: f32 = climate.wind.iter().map(|v| v[1] * v[1]).sum();
        (sum_sq / tile_count as f32).sqrt()
    } else {
        0.0
    };

    info!(
        precip_land_frac,
        precip_land_mean,
        precip_water_mean,
        precip_land_water_ratio,
        moisture_land_mean,
        wind_ns_rms,
        "climate stats"
    );
}

fn log_hydrology_stats(hydrology: &HydrologyData) {
    let lake_count = hydrology.is_lake.iter().filter(|&&v| v).count();
    let creek_count = hydrology
        .river_size
        .iter()
        .filter(|&&v| v == Some(RiverSize::Creek))
        .count();
    let river_count = hydrology
        .river_size
        .iter()
        .filter(|&&v| v == Some(RiverSize::River))
        .count();
    let major_river_count = hydrology
        .river_size
        .iter()
        .filter(|&&v| v == Some(RiverSize::MajorRiver))
        .count();
    let floodplain_count = hydrology.is_floodplain.iter().filter(|&&v| v).count();
    let canyon_count = hydrology.is_canyon.iter().filter(|&&v| v).count();
    let canyon_wall_count = hydrology.is_canyon_wall.iter().filter(|&&v| v).count();

    info!(
        lake_count,
        creek_count,
        river_count,
        major_river_count,
        floodplain_count,
        canyon_count,
        canyon_wall_count,
        "hydrology stats"
    );
}

fn log_biome_stats(biomes: &BiomeData, world: &WorldData) {
    let mut counts = std::collections::BTreeMap::new();
    let mut unassigned_count = 0_usize;

    for &biome in biomes.biome.as_slice() {
        match biome {
            None => unassigned_count += 1,
            Some(handle) => {
                let id = world.biome_registry.id_of(handle);
                *counts.entry(id.to_owned()).or_insert(0_usize) += 1;
            }
        }
    }

    for (id, count) in &counts {
        info!(biome = %id, count, "biome count");
    }
    info!(unassigned_count, "biome stats");
}

fn log_resource_stats(resources: &ResourceData, world: &WorldData) {
    let mut counts = std::collections::BTreeMap::new();

    for &resource in resources.resource.as_slice() {
        if let Some(handle) = resource {
            let id = world.resource_registry.id_of(handle);
            *counts.entry(id.to_owned()).or_insert(0_usize) += 1;
        }
    }

    for (id, count) in &counts {
        info!(resource = %id, count, "resource count");
    }
    info!("resource stats logged");
}
