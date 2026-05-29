//! Automated rendering benchmark for the strategy game.
//!
//! Runs a configurable matrix of (seed × map-size) cases through the Bevy
//! renderer, scripting camera movement and recording frame-time statistics.
//! Results are written to a JSON file and summarised in a terminal table.

#![allow(missing_docs)] // binary crate; CLI help and bench.ron are the user docs.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use config::{BenchConfig, MapSizeArg};

mod camera_path;
mod collector;
mod config;
mod orchestrator;
mod reporter;
mod single_case;

#[derive(Debug, Parser)]
#[command(about = "Automated rendering benchmark for the strategy game")]
struct Args {
    /// RON bench configuration file.
    #[arg(default_value = "tools/mapbench/bench.ron")]
    config: PathBuf,

    /// Directory for JSON result files.
    #[arg(long, default_value = "bench-results")]
    output: String,

    /// Override all seeds (repeatable: --seed 1 --seed 2).
    #[arg(long, value_name = "SEED")]
    seed: Vec<u64>,

    /// Override all map sizes (repeatable: --size small --size medium).
    #[arg(long, value_name = "SIZE")]
    size: Vec<MapSizeArg>,

    /// Override measurement duration in seconds.
    #[arg(long)]
    duration: Option<u32>,

    /// Internal: run one benchmark case. Argument is a JSON-encoded `BenchCase`.
    #[arg(long, hide = true)]
    single_case: Option<String>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    // Single-case mode: called by the orchestrator subprocess, not by users.
    if let Some(case_json) = args.single_case {
        let case = serde_json::from_str(&case_json)
            .map_err(|e| anyhow::anyhow!("failed to parse --single-case JSON: {e}"))?;
        single_case::run_single_case(case);
        return Ok(());
    }

    // Orchestrator mode: load config and run all cases.
    let cfg = if args.config.exists() {
        let content = std::fs::read_to_string(&args.config)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", args.config.display()))?;
        ron::from_str::<BenchConfig>(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", args.config.display()))?
    } else {
        tracing::info!(
            path = %args.config.display(),
            "config file not found, using defaults"
        );
        BenchConfig::default()
    };

    let seed_overrides = if args.seed.is_empty() {
        None
    } else {
        Some(args.seed)
    };
    let size_overrides = if args.size.is_empty() {
        None
    } else {
        Some(args.size)
    };

    orchestrator::run_bench(
        &cfg,
        size_overrides.as_deref(),
        seed_overrides.as_deref(),
        args.duration,
        &args.output,
    )
}
