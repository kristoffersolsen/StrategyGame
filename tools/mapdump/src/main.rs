//! CLI tool for running and inspecting world generation output.
//!
//! Generates a world from a given seed and parameter set, then writes
//! per-stage debug images to an output directory. Used for tuning
//! generation parameters without running the full Bevy application.
//!
//! # Usage
//!
//! ```text
//! mapdump --seed 42 --output /tmp/map-debug/
//! mapdump --seed 42 --output /tmp/map-debug/ --size large --hex-size 6
//! ```
//!
//! # Output
//!
//! All PNGs are written to `<output>/`. Files are prefixed by stage
//! number so they sort in pipeline order.
//!
//! ## Stage 0 (infrastructure validation)
//!
//! - `00_stub_gradient_hex.png` — hex-rasterized viridis gradient,
//!   confirming that the hex rendering pipeline is working.
//! - `00_stub_gradient_px.png` — one-pixel-per-tile version of the
//!   same, used by determinism tests.
//!
//! ## Stage 3.1 — Tectonics step 1: plate seed placement
//!
//! - `31_01_plate_seeds_px.png` — one-pixel-per-tile blank field with
//!   Poisson-disk plate seeds painted as white markers on a black
//!   background. Used to verify: seed count near target, even
//!   distribution, no wrap-seam clustering.
//!
//! ## Stage 3.1 — Tectonics steps 2–3: plate map
//!
//! - `31_03_plate_map_px.png` — one-pixel-per-tile plate-ID field,
//!   each plate a distinct categorical color. Used to verify: full
//!   coverage, organic boundaries, no wrap-seam artifact.
//! - `31_03_plate_map_hex.png` — hex-rasterized version of the same.
//!   Primary visual for tuning `boundary_perturbation_amplitude`.
//!
//! ## Stage 3.1 — Tectonics step 5: boundary classification
//!
//! - `31_05_boundary_types_px.png` — one-pixel-per-tile boundary map.
//!   Interior tiles: black. Convergent: bright red. Divergent: bright
//!   blue. Transform: grey.
//!
//! ## Stage 3.1 — Tectonics steps 6–7: elevation field
//!
//! - `31_07_elevation_px.png` — one-pixel-per-tile elevation field,
//!   normalised to `[0, 1]` for the Viridis colormap. Darker = lower.
//!   Before sea-level correction (the raw post-detail-noise elevation).
//!
//! ## Stage 3.1 — Tectonics step 8: height levels
//!
//! - `31_08_height_levels_px.png` — one-pixel-per-tile discrete height
//!   level (0 = ocean, 1 = coast, 2+ = land, last = mountain) using the
//!   categorical palette. Primary visual for tuning `level_proportions`.
//! - `31_08_height_levels_hex.png` — hex-rasterized version of the same.
//!
//! ## Stage 3.1 — Tectonics step 9: hills
//!
//! - `31_09_hills_px.png` — one-pixel-per-tile hilliness field.
//!   White = hilly, black = flat.

use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tracing::info;
use worldgen::{
    BasicParams, BoundaryType, Field, MapSize, TectonicData, WorldParams,
    debug_viz::{
        Categorical, Colormap, Grayscale, Viridis, dump_field_hex, dump_field_pixels,
        dump_field_with_seed_overlay,
    },
    generate_world,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    info!(seed = args.seed, output = %args.output.display(), "starting mapdump");

    std::fs::create_dir_all(&args.output).with_context(|| {
        format!(
            "failed to create output directory: {}",
            args.output.display()
        )
    })?;

    let params = WorldParams {
        seed: args.seed,
        basic: BasicParams {
            map_size: args.size.into(),
            ..BasicParams::default()
        },
        ..WorldParams::default()
    };

    info!("running generate_world");
    let world = generate_world(&params).context("world generation failed")?;
    let bounds = world.bounds;
    info!(
        width = bounds.width(),
        height = bounds.height(),
        "bounds ready"
    );

    // Stage 0: synthetic gradient for render-pipeline validation.
    #[allow(clippy::cast_precision_loss)]
    let gradient = Field::from_fn(bounds, |coord| {
        coord.r as f32 / (bounds.height() - 1) as f32
    });
    write_stage_0(&gradient, &args.output, args.hex_size)?;

    write_tectonics_plates(&world.tectonics, &params, &args.output, args.hex_size)?;
    write_tectonics_elevation_and_levels(&world.tectonics, &params, &args.output, args.hex_size)?;

    info!("done");
    Ok(())
}

// ------------- Output helpers (one per stage group) -------------

fn write_stage_0(gradient: &Field<f32>, output: &Path, hex_size: u32) -> Result<()> {
    let hex_path = output.join("00_stub_gradient_hex.png");
    info!(path = %hex_path.display(), hex_size, "writing hex PNG");
    dump_field_hex(gradient, &hex_path, |v| v, &Viridis, hex_size)
        .with_context(|| format!("failed to write {}", hex_path.display()))?;

    let px_path = output.join("00_stub_gradient_px.png");
    info!(path = %px_path.display(), "writing pixel PNG");
    dump_field_pixels(gradient, &px_path, |v| v, &Viridis)
        .with_context(|| format!("failed to write {}", px_path.display()))?;

    Ok(())
}

fn write_tectonics_plates(
    tectonics: &TectonicData,
    params: &WorldParams,
    output: &Path,
    hex_size: u32,
) -> Result<()> {
    let seed_count = tectonics.seeds.len();
    info!(
        seed_count,
        target = params.tuning.tectonics.target_plate_count,
        "plate seeds placed"
    );

    let black = Field::filled(tectonics.plate_id.bounds(), 0.0_f32);
    let seeds_path = output.join("31_01_plate_seeds_px.png");
    info!(path = %seeds_path.display(), "writing plate seeds PNG");
    dump_field_with_seed_overlay(
        &black,
        &tectonics.seeds,
        &seeds_path,
        |v| *v,
        &Grayscale,
        [255, 255, 255],
    )
    .with_context(|| format!("failed to write {}", seeds_path.display()))?;

    let plate_count = tectonics.seeds.len();
    info!(plate_count, "noise-perturbed Voronoi partition complete");

    let plates_px_path = output.join("31_03_plate_map_px.png");
    info!(path = %plates_px_path.display(), "writing plate map pixel PNG");
    dump_field_pixels(
        &tectonics.plate_id,
        &plates_px_path,
        f32::from,
        &Categorical,
    )
    .with_context(|| format!("failed to write {}", plates_px_path.display()))?;

    let plates_hex_path = output.join("31_03_plate_map_hex.png");
    info!(path = %plates_hex_path.display(), hex_size, "writing plate map hex PNG");
    dump_field_hex(
        &tectonics.plate_id,
        &plates_hex_path,
        f32::from,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", plates_hex_path.display()))?;

    let convergent_count = tectonics
        .boundary_type
        .iter()
        .filter(|v| **v == Some(BoundaryType::Convergent))
        .count();
    let divergent_count = tectonics
        .boundary_type
        .iter()
        .filter(|v| **v == Some(BoundaryType::Divergent))
        .count();
    let transform_count = tectonics
        .boundary_type
        .iter()
        .filter(|v| **v == Some(BoundaryType::Transform))
        .count();
    info!(
        convergent = convergent_count,
        divergent = divergent_count,
        transform = transform_count,
        "boundary classification complete"
    );

    let boundary_path = output.join("31_05_boundary_types_px.png");
    info!(path = %boundary_path.display(), "writing boundary types PNG");
    dump_field_pixels(
        &tectonics.boundary_type,
        &boundary_path,
        |v| match v {
            None => 0.0,
            Some(BoundaryType::Transform) => 1.0,
            Some(BoundaryType::Divergent) => 2.0,
            Some(BoundaryType::Convergent) => 3.0,
        },
        &BoundaryCmap,
    )
    .with_context(|| format!("failed to write {}", boundary_path.display()))?;

    Ok(())
}

fn write_tectonics_elevation_and_levels(
    tectonics: &TectonicData,
    params: &WorldParams,
    output: &Path,
    hex_size: u32,
) -> Result<()> {
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
    info!(
        min = elev_min,
        max = elev_max,
        range = elev_range,
        "elevation field stats"
    );

    let elev_path = output.join("31_07_elevation_px.png");
    info!(path = %elev_path.display(), "writing elevation PNG");
    dump_field_pixels(
        &tectonics.elevation,
        &elev_path,
        |v| {
            if elev_range > 0.0 {
                (v - elev_min) / elev_range
            } else {
                0.5
            }
        },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", elev_path.display()))?;

    let n = usize::from(params.tuning.tectonics.land_level_count) + 3;
    let mut level_counts = vec![0usize; n];
    for &level in &tectonics.height_level {
        if (level as usize) < n {
            level_counts[level as usize] += 1;
        }
    }
    let total_tiles = tectonics.height_level.len();
    #[allow(clippy::cast_precision_loss)]
    let ocean_pct = level_counts[0] as f32 / total_tiles as f32 * 100.0;
    #[allow(clippy::cast_precision_loss)]
    let coast_pct = level_counts[1] as f32 / total_tiles as f32 * 100.0;
    info!(
        ocean = format!("{ocean_pct:.1}%"),
        coast = format!("{coast_pct:.1}%"),
        "height levels assigned"
    );

    let levels_px_path = output.join("31_08_height_levels_px.png");
    info!(path = %levels_px_path.display(), "writing height levels pixel PNG");
    dump_field_pixels(
        &tectonics.height_level,
        &levels_px_path,
        f32::from,
        &HeightLevelCmap,
    )
    .with_context(|| format!("failed to write {}", levels_px_path.display()))?;

    let levels_hex_path = output.join("31_08_height_levels_hex.png");
    info!(path = %levels_hex_path.display(), hex_size, "writing height levels hex PNG");
    dump_field_hex(
        &tectonics.height_level,
        &levels_hex_path,
        f32::from,
        &HeightLevelCmap,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", levels_hex_path.display()))?;

    let hilly_count = tectonics.is_hilly.iter().filter(|&&h| h).count();
    #[allow(clippy::cast_precision_loss)]
    let hilly_fraction = hilly_count as f32 / total_tiles as f32 * 100.0;
    info!(
        hilly_count,
        hilly_fraction = format!("{hilly_fraction:.1}%"),
        "hills classified"
    );

    let hills_path = output.join("31_09_hills_px.png");
    info!(path = %hills_path.display(), "writing hills PNG");
    dump_field_pixels(
        &tectonics.is_hilly,
        &hills_path,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
    )
    .with_context(|| format!("failed to write {}", hills_path.display()))?;

    Ok(())
}

// ------------- Custom colormaps for mapdump -------------

/// Four-colour map for boundary types.
///
/// 0 (interior) → near-black, 1 (transform) → mid-grey,
/// 2 (divergent) → blue, 3 (convergent) → red.
struct BoundaryCmap;

/// Six-colour map for height levels produced by Stage 3.1 step 8.
///
/// Levels are passed as floats `0.0`–`5.0` (from the `u8` height-level
/// field via `f32::from`):
///
/// - 0 (ocean)   → dark blue
/// - 1 (coast)   → light blue
/// - 2 (low land)  → orange
/// - 3 (mid land)  → orange-red
/// - 4 (high land) → red
/// - 5 (mountain)  → deep red
///
/// Matches the default `land_level_count = 3` (6 total levels). If that
/// parameter is changed, extend this palette to cover additional levels.
struct HeightLevelCmap;

impl Colormap for BoundaryCmap {
    fn color(&self, value: f32) -> [u8; 3] {
        // Quantise to the four boundary levels.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        match (value.round() as u8).min(3) {
            0 => [30, 30, 30],    // interior — near-black
            1 => [140, 140, 140], // transform — grey
            2 => [60, 120, 220],  // divergent — blue
            _ => [220, 60, 60],   // convergent — red
        }
    }
}

impl Colormap for HeightLevelCmap {
    fn color(&self, value: f32) -> [u8; 3] {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        match (value.round() as u8).min(5) {
            0 => [15, 55, 120],  // ocean      — dark blue
            1 => [80, 155, 215], // coast      — light blue
            2 => [230, 155, 55], // low land   — orange
            3 => [205, 85, 30],  // mid land   — orange-red
            4 => [175, 40, 20],  // high land  — red
            _ => [130, 15, 15],  // mountain   — deep red
        }
    }
}

// ------------- CLI argument types -------------

/// Generates world-generation debug PNGs from a seed and parameter set.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Master RNG seed. The same seed always produces identical output.
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Directory to write PNG output files into. Created if absent.
    #[arg(long, default_value = "mapdump-output")]
    output: PathBuf,

    /// Map size archetype. Controls tile count; see `WorldParams` docs.
    #[arg(long, default_value = "medium")]
    size: SizeArg,

    /// Circumradius of each hex tile in the rasterized output, in pixels.
    /// Larger values produce crisper images but larger files.
    /// At `medium` map size: size 8 → ~20 MB, size 4 → ~5 MB.
    #[arg(long, default_value_t = 8)]
    hex_size: u32,
}

/// Map size argument. Maps directly to [`MapSize`].
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SizeArg {
    Small,
    Standard,
    Medium,
    Large,
    Huge,
}

impl From<SizeArg> for MapSize {
    fn from(arg: SizeArg) -> Self {
        match arg {
            SizeArg::Small => MapSize::Small,
            SizeArg::Standard => MapSize::Standard,
            SizeArg::Medium => MapSize::Medium,
            SizeArg::Large => MapSize::Large,
            SizeArg::Huge => MapSize::Huge,
        }
    }
}
