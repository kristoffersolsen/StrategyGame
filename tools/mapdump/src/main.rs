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
//! All PNGs are written to `<output>/`. Each stage of the generation
//! pipeline will add its own files here as the stages are implemented.
//! Currently (Stage 2 stub) only the synthetic gradient files are written:
//!
//! - `00_stub_gradient_hex.png` — hex-rasterized viridis gradient, for
//!   visual confirmation that the rendering pipeline is working.
//! - `00_stub_gradient_px.png` — one-pixel-per-tile version of the same,
//!   for use in determinism tests.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tracing::info;
use worldgen::{
    BasicParams, Field, MapSize, WorldParams,
    debug_viz::{Viridis, dump_field_hex, dump_field_pixels},
    generate_world,
};

fn main() -> Result<()> {
    // Initialise tracing to stderr so it doesn't interfere with stdout
    // if a caller pipes mapdump output.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    info!(seed = args.seed, output = %args.output.display(), "starting mapdump");

    // Create output directory, including any missing parent directories.
    std::fs::create_dir_all(&args.output).with_context(|| {
        format!(
            "failed to create output directory: {}",
            args.output.display()
        )
    })?;

    // Build WorldParams from CLI arguments.
    let params = WorldParams {
        seed: args.seed,
        basic: BasicParams {
            map_size: args.size.into(),
            ..BasicParams::default()
        },
        ..WorldParams::default()
    };

    // Run the generation pipeline. Currently a stub that returns only
    // MapBounds; real stage output is added in Stage 3.
    info!("running generate_world");
    let world = generate_world(&params).context("world generation failed")?;
    let bounds = world.bounds;
    info!(
        width = bounds.width(),
        height = bounds.height(),
        "bounds ready"
    );

    // Construct a synthetic gradient field for visual validation of the
    // rendering pipeline. Each tile's value is its normalised row index
    // (0.0 at the north pole, 1.0 at the south pole), giving a smooth
    // viridis gradient from purple to yellow across the map.
    //
    // The casts below are safe: map heights are at most 250 (Huge variant),
    // well within f32's 24-bit mantissa precision.
    #[allow(clippy::cast_precision_loss)]
    let gradient = Field::from_fn(bounds, |coord| {
        coord.r as f32 / (bounds.height() - 1) as f32
    });

    // Write hex-rasterized output for human inspection.
    let hex_path = args.output.join("00_stub_gradient_hex.png");
    info!(path = %hex_path.display(), hex_size = args.hex_size, "writing hex PNG");
    dump_field_hex(&gradient, &hex_path, |v| v, &Viridis, args.hex_size)
        .with_context(|| format!("failed to write {}", hex_path.display()))?;

    // Write pixel-per-tile output for determinism test hashing.
    let px_path = args.output.join("00_stub_gradient_px.png");
    info!(path = %px_path.display(), "writing pixel PNG");
    dump_field_pixels(&gradient, &px_path, |v| v, &Viridis)
        .with_context(|| format!("failed to write {}", px_path.display()))?;

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
