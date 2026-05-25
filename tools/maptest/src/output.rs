//! Image writing helpers shared by the `run` and `sweep` subcommands.
//!
//! `write_world_outputs` writes all eight Stage 3.1 debug images for a
//! single world into a structured output directory. The caller supplies a
//! `label` string that becomes the image filename stem, allowing `run` to
//! use `seed_N` and `sweep` to use `v<value>_seed<N>`.
//!
//! Call `ensure_output_dirs` once per base directory (from the main
//! thread, before spawning rayon tasks) to avoid race conditions when
//! multiple workers write to the same set of subdirectories.

use std::path::Path;

use anyhow::{Context, Result};
use worldgen::{
    BoundaryType, Field, TectonicData,
    debug_viz::{
        BoundaryCmap, Categorical, Grayscale, HeightLevelCmap, Viridis, dump_field_hex,
        dump_field_pixels, dump_field_with_seed_overlay,
    },
};

// ------------- Subdirectory names (stable across run and sweep) -------------

pub(crate) const SUBDIR_SEEDS: &str = "seeds";
pub(crate) const SUBDIR_PLATE_PX: &str = "plate_map_px";
pub(crate) const SUBDIR_PLATE_HEX: &str = "plate_map_hex";
pub(crate) const SUBDIR_BOUNDARY: &str = "boundary_types";
pub(crate) const SUBDIR_ELEVATION: &str = "elevation";
pub(crate) const SUBDIR_LEVELS_PX: &str = "height_levels_px";
pub(crate) const SUBDIR_LEVELS_HEX: &str = "height_levels_hex";
pub(crate) const SUBDIR_HILLS: &str = "hills";

pub(crate) const ALL_SUBDIRS: &[&str] = &[
    SUBDIR_SEEDS,
    SUBDIR_PLATE_PX,
    SUBDIR_PLATE_HEX,
    SUBDIR_BOUNDARY,
    SUBDIR_ELEVATION,
    SUBDIR_LEVELS_PX,
    SUBDIR_LEVELS_HEX,
    SUBDIR_HILLS,
];

// ------------- Directory preparation -------------

/// Creates all eight output subdirectories under `base_dir`.
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

/// Writes all eight Stage 3.1 debug images for `tectonics` into `base_dir`.
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
    base_dir: &Path,
    label: &str,
    hex_size: u32,
) -> Result<()> {
    write_seeds(tectonics, base_dir, label)?;
    write_plate_map(tectonics, base_dir, label, hex_size)?;
    write_boundary_types(tectonics, base_dir, label)?;
    write_elevation(tectonics, base_dir, label)?;
    write_height_levels(tectonics, base_dir, label, hex_size)?;
    write_hills(tectonics, base_dir, label)?;
    Ok(())
}

// ------------- Per-output helpers -------------

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
    let px_path = base_dir.join(SUBDIR_PLATE_PX).join(format!("{label}.png"));
    dump_field_pixels(&tectonics.plate_id, &px_path, f32::from, &Categorical)
        .with_context(|| format!("failed to write {}", px_path.display()))?;

    let hex_path = base_dir.join(SUBDIR_PLATE_HEX).join(format!("{label}.png"));
    dump_field_hex(
        &tectonics.plate_id,
        &hex_path,
        f32::from,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", hex_path.display()))
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
) -> Result<()> {
    let px_path = base_dir.join(SUBDIR_LEVELS_PX).join(format!("{label}.png"));
    dump_field_pixels(
        &tectonics.height_level,
        &px_path,
        f32::from,
        &HeightLevelCmap,
    )
    .with_context(|| format!("failed to write {}", px_path.display()))?;

    let hex_path = base_dir
        .join(SUBDIR_LEVELS_HEX)
        .join(format!("{label}.png"));
    dump_field_hex(
        &tectonics.height_level,
        &hex_path,
        f32::from,
        &HeightLevelCmap,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", hex_path.display()))
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
