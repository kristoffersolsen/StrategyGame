//! World-generation parameter tuning and visual regression tool.
//!
//! Two subcommands:
//!
//! - **`run`** — generate all Stage 3.1 and 3.2 debug images for a fixed
//!   parameter set across a range of seeds. Configurable via RON presets.
//!
//! - **`sweep`** — vary a single [`TuningParams`] scalar field across an
//!   evenly-spaced range while holding all other parameters fixed. Runs
//!   (value, seed) pairs in parallel via rayon.
//!
//! # Parameter files
//!
//! Both subcommands accept `--params <file.ron>`, which loads a
//! serialized [`TuningParams`] from a RON file. If omitted, all
//! parameters use their defaults. See `presets/default.ron` for the
//! full set of fields and their defaults.
//!
//! # Examples
//!
//! ```text
//! # Regenerate the standard cross-seed comparison (seeds 1–8, defaults):
//! cargo run --release --bin maptest -- run
//!
//! # Same with a custom preset:
//! cargo run --release --bin maptest -- run --params tools/maptest/presets/default.ron
//!
//! # Sweep mountain_elevation_boost from 0.4 to 2.0 in 5 steps, seeds 1–4:
//! cargo run --release --bin maptest -- sweep \
//!     --param mountain_elevation_boost --min 0.4 --max 2.0 --steps 5
//!
//! # Sweep with a non-default base config and 8 parallel jobs:
//! cargo run --release --bin maptest -- sweep \
//!     --param wind_amplitude --min 0.05 --max 0.25 --steps 9 \
//!     --params tools/maptest/presets/default.ron --jobs 8
//! ```

#![allow(missing_docs)] // binary crate — no public API to document

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use tracing::info;
use worldgen::{BasicParams, MapSize, TuningParams, WorldParams, generate_world};

mod output;
mod sweep;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Run(args) => run(&args),
        Command::Sweep(args) => sweep::run(&args),
    }
}

// ------------- CLI structure -------------

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate all debug images for a fixed parameter set across seeds.
    Run(RunArgs),
    /// Sweep a single parameter across a range of values.
    Sweep(sweep::SweepArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// Seeds to generate. Defaults to seeds 1–8.
    #[arg(long, num_args(1..), default_values_t = [1u64, 2, 3, 4, 5, 6, 7, 8])]
    seeds: Vec<u64>,

    /// Optional RON file containing a `TuningParams` struct.
    #[arg(long)]
    params: Option<PathBuf>,

    /// Output directory. Defaults to `tmp/tests/mapgen`.
    #[arg(long, default_value = "tmp/tests/mapgen")]
    output: PathBuf,

    /// Map size archetype.
    #[arg(long, default_value = "medium")]
    size: SizeArg,

    /// Hex tile circumradius in the rasterized output, in pixels.
    #[arg(long, default_value_t = 16)]
    hex_size: u32,
}

// ------------- run subcommand -------------

fn run(args: &RunArgs) -> Result<()> {
    let tuning = load_tuning_params(args.params.as_deref())?;
    output::ensure_output_dirs(&args.output)?;

    info!(
        seeds = ?args.seeds,
        output = %args.output.display(),
        "starting run"
    );

    for &seed in &args.seeds {
        info!(seed, "generating world");

        let mut params = WorldParams {
            seed,
            basic: BasicParams {
                map_size: args.size.into(),
                ..BasicParams::default()
            },
            ..WorldParams::default()
        };
        params.tuning = tuning.clone();

        let world = generate_world(&params)
            .with_context(|| format!("generation failed for seed {seed}"))?;

        let label = format!("seed_{seed}");
        output::write_world_outputs(
            &world.tectonics,
            &world.climate,
            &args.output,
            &label,
            args.hex_size,
            params.tuning.tectonics.land_level_count,
        )
        .with_context(|| format!("image write failed for seed {seed}"))?;
    }

    info!(output = %args.output.display(), "run complete");
    Ok(())
}

// ------------- Shared parameter loading -------------

pub(crate) fn load_tuning_params(path: Option<&Path>) -> Result<TuningParams> {
    let Some(p) = path else {
        return Ok(TuningParams::defaults());
    };
    let content = std::fs::read_to_string(p)
        .with_context(|| format!("failed to read params file: {}", p.display()))?;
    ron::from_str(&content).with_context(|| {
        format!(
            "failed to parse params file as TuningParams: {}",
            p.display()
        )
    })
}

// ------------- Map size arg (shared with sweep) -------------

/// Map size archetype. Maps directly to [`MapSize`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum SizeArg {
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
