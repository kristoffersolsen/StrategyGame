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
    BoundaryType, ClimateData, Field, TectonicData,
    debug_viz::{
        BoundaryCmap, Categorical, Grayscale, HeightLevelCmap, Viridis, dump_field_hex,
        dump_field_pixels, dump_field_with_seed_overlay,
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
    tectonics: &TectonicData,
    climate: &ClimateData,
    base_dir: &Path,
    label: &str,
    hex_size: u32,
    land_level_count: u8,
) -> Result<()> {
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
    log_climate_stats(tectonics, climate);
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
