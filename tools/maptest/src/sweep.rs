//! Parameter-range sweep subcommand.
//!
//! Varies a single [`TectonicsParams`] scalar field across an evenly-spaced
//! range of values, generating the full set of debug images for each
//! (value, seed) pair. All pairs run in parallel via [`rayon`].

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use tracing::info;
use worldgen::{BasicParams, TectonicsParams, WorldParams, generate_world};

use crate::output;

// ------------- CLI args -------------

/// Sweep a single `TectonicsParams` field across a range of values.
#[derive(Debug, Parser)]
pub(crate) struct SweepArgs {
    /// `TectonicsParams` field name in `snake_case` to vary.
    /// Vec fields (`level_proportions`, `mineral_weights`) are not supported.
    #[arg(long)]
    pub(crate) param: String,

    /// Range minimum (inclusive).
    #[arg(long)]
    pub(crate) min: f64,

    /// Range maximum (inclusive).
    #[arg(long)]
    pub(crate) max: f64,

    /// Number of evenly-spaced values to generate (minimum 2).
    #[arg(long)]
    pub(crate) steps: usize,

    /// Seeds to generate for each parameter value.
    #[arg(long, num_args(1..), default_values_t = [1u64, 2, 3, 4])]
    pub(crate) seeds: Vec<u64>,

    /// Optional RON file with base `TectonicsParams`. All fields not
    /// being swept take their values from this file (or defaults).
    #[arg(long)]
    pub(crate) params: Option<PathBuf>,

    /// Output directory. Defaults to `tmp/tests/tuning/<param>`.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,

    /// Map size archetype.
    #[arg(long, default_value = "medium")]
    pub(crate) size: crate::SizeArg,

    /// Hex tile circumradius in the rasterized output, in pixels.
    #[arg(long, default_value_t = 16)]
    pub(crate) hex_size: u32,

    /// Maximum number of parallel jobs. Defaults to the rayon thread pool size.
    #[arg(long)]
    pub(crate) jobs: Option<usize>,
}

// ------------- Entry point -------------

pub(crate) fn run(args: &SweepArgs) -> Result<()> {
    anyhow::ensure!(args.steps >= 2, "--steps must be at least 2");

    let base_tectonics = crate::load_tectonics_params(args.params.as_deref())?;

    // Validate the param name up front (before any generation work).
    {
        let mut probe = base_tectonics.clone();
        set_param(&mut probe, &args.param, args.min)?;
    }

    let output_dir = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("tmp/tests/tuning").join(&args.param));
    output::ensure_output_dirs(&output_dir)?;

    let values = linspace(args.min, args.max, args.steps);

    if let Some(n) = args.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .context("failed to configure rayon thread pool")?;
    }

    // Collect all (value, seed) tasks.
    let tasks: Vec<(f64, u64)> = values
        .iter()
        .flat_map(|&v| args.seeds.iter().map(move |&s| (v, s)))
        .collect();

    info!(
        param = %args.param,
        steps = args.steps,
        seeds = args.seeds.len(),
        tasks = tasks.len(),
        output = %output_dir.display(),
        "starting sweep"
    );

    tasks
        .par_iter()
        .try_for_each(|&(val, seed)| -> Result<()> {
            let mut tectonics = base_tectonics.clone();
            set_param(&mut tectonics, &args.param, val)?;

            let mut params = WorldParams {
                seed,
                basic: BasicParams {
                    map_size: args.size.into(),
                    ..BasicParams::default()
                },
                ..WorldParams::default()
            };
            params.tuning.tectonics = tectonics;

            let world = generate_world(&params).with_context(|| {
                format!(
                    "generation failed: param={}, value={val}, seed={seed}",
                    args.param
                )
            })?;

            let label = format!("v{val:.4}_{seed}");
            output::write_world_outputs(&world.tectonics, &output_dir, &label, args.hex_size)
                .with_context(|| format!("image write failed: value={val}, seed={seed}"))
        })?;

    info!(
        tasks = tasks.len(),
        output = %output_dir.display(),
        "sweep complete"
    );
    Ok(())
}

// ------------- Range generation -------------

fn linspace(min: f64, max: f64, steps: usize) -> Vec<f64> {
    if steps == 1 {
        return vec![min];
    }
    #[allow(clippy::cast_precision_loss)]
    (0..steps)
        .map(|i| min + i as f64 * (max - min) / (steps - 1) as f64)
        .collect()
}

// ------------- Field assignment by name -------------

/// Sets a single scalar field of `t` by its `snake_case` name.
///
/// Vec fields (`level_proportions`, `mineral_weights`) cannot be swept
/// as scalars; provide them via a `--params` config file instead.
///
/// # Errors
///
/// Returns an error if `name` is unknown, is a Vec field, or if the
/// value is out of range for the field's type.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(crate) fn set_param(t: &mut TectonicsParams, name: &str, value: f64) -> Result<()> {
    match name {
        // Integer fields — round before casting.
        "target_plate_count" => t.target_plate_count = value.round() as u8,
        "land_level_count" => t.land_level_count = value.round() as u8,
        "detail_noise_octaves" => t.detail_noise_octaves = value.round() as u8,
        "hill_influence_radius" => t.hill_influence_radius = value.round() as u32,
        // f32 fields.
        "boundary_perturbation_amplitude" => t.boundary_perturbation_amplitude = value as f32,
        "motion_magnitude_min" => t.motion_magnitude_min = value as f32,
        "motion_magnitude_max" => t.motion_magnitude_max = value as f32,
        "convergence_threshold" => t.convergence_threshold = value as f32,
        "continental_base_elevation" => t.continental_base_elevation = value as f32,
        "oceanic_base_elevation" => t.oceanic_base_elevation = value as f32,
        "intra_plate_noise_amplitude" => t.intra_plate_noise_amplitude = value as f32,
        "intra_plate_noise_frequency" => t.intra_plate_noise_frequency = value as f32,
        "boundary_falloff_sigma" => t.boundary_falloff_sigma = value as f32,
        "mountain_elevation_boost" => t.mountain_elevation_boost = value as f32,
        "coastal_mountain_boost" => t.coastal_mountain_boost = value as f32,
        "subduction_trench_penalty" => t.subduction_trench_penalty = value as f32,
        "island_arc_boost" => t.island_arc_boost = value as f32,
        "rift_valley_penalty" => t.rift_valley_penalty = value as f32,
        "mid_ocean_ridge_boost" => t.mid_ocean_ridge_boost = value as f32,
        "detail_noise_amplitude" => t.detail_noise_amplitude = value as f32,
        "detail_noise_frequency" => t.detail_noise_frequency = value as f32,
        "hill_near_boundary_threshold" => t.hill_near_boundary_threshold = value as f32,
        "hill_interior_threshold" => t.hill_interior_threshold = value as f32,
        "hill_noise_frequency" => t.hill_noise_frequency = value as f32,
        // Vec fields — not sweepable as scalars.
        "level_proportions" | "mineral_weights" => anyhow::bail!(
            "`{name}` is a Vec field; set it via --params <file.ron> instead of --param/--range"
        ),
        _ => anyhow::bail!(
            "unknown TectonicsParams field: `{name}`\n\
             Sweepable fields: target_plate_count, boundary_perturbation_amplitude, \
             motion_magnitude_min, motion_magnitude_max, convergence_threshold, \
             continental_base_elevation, oceanic_base_elevation, \
             intra_plate_noise_amplitude, intra_plate_noise_frequency, \
             boundary_falloff_sigma, mountain_elevation_boost, coastal_mountain_boost, \
             subduction_trench_penalty, island_arc_boost, rift_valley_penalty, \
             mid_ocean_ridge_boost, detail_noise_amplitude, detail_noise_frequency, \
             detail_noise_octaves, hill_influence_radius, hill_near_boundary_threshold, \
             hill_interior_threshold, hill_noise_frequency"
        ),
    }
    Ok(())
}
