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

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tracing::info;
use worldgen::{
    BasicParams, Field, MapSize, WorldParams,
    debug_viz::{
        Categorical, Grayscale, Viridis, dump_field_hex, dump_field_pixels,
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

    // ------------- Stage 0: infrastructure stub -------------

    // Synthetic gradient for render-pipeline validation.
    #[allow(clippy::cast_precision_loss)]
    let gradient = Field::from_fn(bounds, |coord| {
        coord.r as f32 / (bounds.height() - 1) as f32
    });

    let hex_path = args.output.join("00_stub_gradient_hex.png");
    info!(path = %hex_path.display(), hex_size = args.hex_size, "writing hex PNG");
    dump_field_hex(&gradient, &hex_path, |v| v, &Viridis, args.hex_size)
        .with_context(|| format!("failed to write {}", hex_path.display()))?;

    let px_path = args.output.join("00_stub_gradient_px.png");
    info!(path = %px_path.display(), "writing pixel PNG");
    dump_field_pixels(&gradient, &px_path, |v| v, &Viridis)
        .with_context(|| format!("failed to write {}", px_path.display()))?;

    // ------------- Stage 3.1, step 1: plate seeds -------------

    let seed_count = world.tectonics.seeds.len();
    info!(
        seed_count,
        target = params.tuning.tectonics.target_plate_count,
        "plate seeds placed"
    );

    // Black base field; seeds painted white so they stand out clearly.
    let black = Field::filled(bounds, 0.0_f32);
    let seeds_path = args.output.join("31_01_plate_seeds_px.png");
    info!(path = %seeds_path.display(), "writing plate seeds PNG");
    dump_field_with_seed_overlay(
        &black,
        &world.tectonics.seeds,
        &seeds_path,
        |v| *v,
        &Grayscale,
        [255, 255, 255], // white markers
    )
    .with_context(|| format!("failed to write {}", seeds_path.display()))?;

    // ------------- Stage 3.1, steps 2–3: plate map -------------

    let plate_count = world.tectonics.seeds.len();
    info!(plate_count, "noise-perturbed Voronoi partition complete");

    let plates_px_path = args.output.join("31_03_plate_map_px.png");
    info!(path = %plates_px_path.display(), "writing plate map pixel PNG");
    dump_field_pixels(
        &world.tectonics.plate_id,
        &plates_px_path,
        f32::from,
        &Categorical,
    )
    .with_context(|| format!("failed to write {}", plates_px_path.display()))?;

    let plates_hex_path = args.output.join("31_03_plate_map_hex.png");
    info!(path = %plates_hex_path.display(), hex_size = args.hex_size, "writing plate map hex PNG");
    dump_field_hex(
        &world.tectonics.plate_id,
        &plates_hex_path,
        f32::from,
        &Categorical,
        args.hex_size,
    )
    .with_context(|| format!("failed to write {}", plates_hex_path.display()))?;

    info!("done");
    Ok(())
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
