//! Parameter-range sweep subcommand.
//!
//! Varies a single [`TuningParams`] scalar field across an evenly-spaced
//! range of values, generating the full set of debug images for each
//! (value, seed) pair. All pairs run in parallel via [`rayon`].

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use tracing::info;
use worldgen::{
    BasicParams, BiomeParams, TectonicsParams, TuningParams, WorldParams, generate_world,
};

use crate::output;

// ------------- CLI args -------------

/// Sweep a single `TuningParams` field across a range of values.
#[derive(Debug, Parser)]
pub(crate) struct SweepArgs {
    /// Field name in `snake_case` to vary (e.g. `mountain_elevation_boost`,
    /// `wind_amplitude`). Vec and bool fields are not sweepable.
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

    /// Optional RON file with base `TuningParams`. All fields not being
    /// swept take their values from this file (or defaults).
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

    let base_tuning = crate::load_tuning_params(args.params.as_deref())?;

    // Validate the param name up front (before any generation work).
    {
        let mut probe = base_tuning.clone();
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
            let mut tuning = base_tuning.clone();
            set_param(&mut tuning, &args.param, val)?;

            let mut params = WorldParams {
                seed,
                basic: BasicParams {
                    map_size: args.size.into(),
                    ..BasicParams::default()
                },
                ..WorldParams::default()
            };
            params.tuning = tuning;

            let world = generate_world(&params).with_context(|| {
                format!(
                    "generation failed: param={}, value={val}, seed={seed}",
                    args.param
                )
            })?;

            let label = format!("v{val:.4}_{seed}");
            output::write_world_outputs(
                &world,
                &output_dir,
                &label,
                args.hex_size,
                params.tuning.tectonics.land_level_count,
            )
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

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn set_tectonics_param(t: &mut TectonicsParams, name: &str, value: f64) -> Result<bool> {
    match name {
        // Integer fields — round before casting.
        "target_plate_count" => t.target_plate_count = value.round() as u8,
        "land_level_count" => t.land_level_count = value.round() as u8,
        "detail_noise_octaves" => t.detail_noise_octaves = value.round() as u8,
        "hill_influence_radius" => t.hill_influence_radius = value.round() as u32,
        // f32 fields.
        "motion_magnitude_min" => t.motion_magnitude_min = value as f32,
        "motion_magnitude_max" => t.motion_magnitude_max = value as f32,
        "plate_growth_rate_range" => t.plate_growth_rate_range = value as f32,
        "clumping_index" => t.clumping_index = value as f32,
        "convergence_threshold" => t.convergence_threshold = value as f32,
        "continental_base_elevation" => t.continental_base_elevation = value as f32,
        "oceanic_base_elevation" => t.oceanic_base_elevation = value as f32,
        "intra_plate_noise_amplitude" => t.intra_plate_noise_amplitude = value as f32,
        "intra_plate_noise_frequency" => t.intra_plate_noise_frequency = value as f32,
        "boundary_falloff_sigma" => t.boundary_falloff_sigma = value as f32,
        "boundary_strength_noise_amplitude" => {
            t.boundary_strength_noise_amplitude = value as f32;
        }
        "boundary_strength_noise_frequency" => {
            t.boundary_strength_noise_frequency = value as f32;
        }
        "boundary_sigma_noise_amplitude" => t.boundary_sigma_noise_amplitude = value as f32,
        "boundary_sigma_noise_frequency" => t.boundary_sigma_noise_frequency = value as f32,
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
        "mountain_pass_threshold" => t.mountain_pass_threshold = value as f32,
        "mountain_pass_noise_frequency" => t.mountain_pass_noise_frequency = value as f32,
        // Vec fields — not sweepable as scalars.
        "level_proportions" | "mineral_weights" => anyhow::bail!(
            "`{name}` is a Vec field; set it via --params <file.ron> instead of --param/--range"
        ),
        "mountain_pass_neighbor_aware" => anyhow::bail!(
            "`mountain_pass_neighbor_aware` is a bool; set it via --params <file.ron>"
        ),
        "mountain_pass_min_mountain_neighbors" => {
            t.mountain_pass_min_mountain_neighbors = value.round().clamp(0.0, 6.0) as u8;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn set_biome_param(biomes: &mut BiomeParams, name: &str, value: f64) -> Result<bool> {
    match name {
        "hydrology_override_strength" => biomes.hydrology_override_strength = value as f32,
        "smoothing_passes" => biomes.smoothing_passes = value.round() as u8,
        "forest_noise_frequency" => biomes.forest_noise_frequency = value as f32,
        "forest_noise_amplitude" => biomes.forest_noise_amplitude = value as f32,
        "temperature_bin_count" | "moisture_bin_count" => anyhow::bail!(
            "`{name}` is coupled to the biome table file; edit the table and params together \
             instead of sweeping it as a scalar"
        ),
        _ => return Ok(false),
    }
    Ok(true)
}

/// Sets a single scalar field of `tuning` by its `snake_case` name.
///
/// Vec and bool fields cannot be swept as scalars; provide them via a
/// `--params` config file instead.
///
/// # Errors
///
/// Returns an error if `name` is unknown, is a non-scalar field, or if
/// the value is out of range for the field's type.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(crate) fn set_param(tuning: &mut TuningParams, name: &str, value: f64) -> Result<()> {
    if set_tectonics_param(&mut tuning.tectonics, name, value)? {
        return Ok(());
    }
    if set_biome_param(&mut tuning.biomes, name, value)? {
        return Ok(());
    }

    let c = &mut tuning.climate;
    let h = &mut tuning.hydrology;
    match name {
        // ---------- ClimateParams ----------
        "equator_temperature" => c.equator_temperature = value as f32,
        "pole_temperature" => c.pole_temperature = value as f32,
        "elevation_lapse_rate" => c.elevation_lapse_rate = value as f32,
        "climate_band_offset" => c.climate_band_offset = value as f32,
        "wind_amplitude" => c.wind_amplitude = value as f32,
        "wind_noise_amplitude" => c.wind_noise_amplitude = value as f32,
        "wind_noise_frequency" => c.wind_noise_frequency = value as f32,
        "evaporation_rate" => c.evaporation_rate = value as f32,
        "evaporation_temp_floor" => c.evaporation_temp_floor = value as f32,
        "evaporation_temp_scale" => c.evaporation_temp_scale = value as f32,
        "iteration_count_factor" => c.iteration_count_factor = value as f32,
        "iteration_count_min" => c.iteration_count_min = value.round() as u32,
        "base_precipitation_rate" => c.base_precipitation_rate = value as f32,
        "max_precipitation_fraction" => c.max_precipitation_fraction = value as f32,
        "temp_capacity_offset" => c.temp_capacity_offset = value as f32,
        "temp_capacity_scale" => c.temp_capacity_scale = value as f32,
        "orographic_factor" => c.orographic_factor = value as f32,
        "coastal_moisture_factor" => c.coastal_moisture_factor = value as f32,
        "evapotranspiration_rate" => c.evapotranspiration_rate = value as f32,
        "moisture_cap" => c.moisture_cap = value as f32,
        "diffusion_rate" => c.diffusion_rate = value as f32,

        // ---------- HydrologyParams ----------
        "lake_basin_area_threshold" => h.lake_basin_area_threshold = value.round() as u32,
        "river_flow_threshold" => h.river_flow_threshold = value as f32,
        "major_river_flow_threshold" => h.major_river_flow_threshold = value as f32,
        "floodplain_elevation_tolerance" => h.floodplain_elevation_tolerance = value as f32,
        "canyon_flow_slope_threshold" => h.canyon_flow_slope_threshold = value as f32,
        "canyon_depth" => h.canyon_depth = value.round() as u8,

        _ => anyhow::bail!(
            "unknown TuningParams field: `{name}`\n\
             Tectonics: target_plate_count, motion_magnitude_min, motion_magnitude_max, \
             plate_growth_rate_range, clumping_index, convergence_threshold, \
             continental_base_elevation, oceanic_base_elevation, \
             intra_plate_noise_amplitude, intra_plate_noise_frequency, boundary_falloff_sigma, \
             boundary_strength_noise_amplitude, boundary_strength_noise_frequency, \
             boundary_sigma_noise_amplitude, boundary_sigma_noise_frequency, \
             mountain_elevation_boost, coastal_mountain_boost, subduction_trench_penalty, \
             island_arc_boost, rift_valley_penalty, mid_ocean_ridge_boost, \
             detail_noise_amplitude, detail_noise_frequency, detail_noise_octaves, \
             hill_influence_radius, hill_near_boundary_threshold, hill_interior_threshold, \
             hill_noise_frequency, mountain_pass_threshold, mountain_pass_noise_frequency\n\
             Climate: equator_temperature, pole_temperature, elevation_lapse_rate, \
             climate_band_offset, wind_amplitude, wind_noise_amplitude, wind_noise_frequency, \
             evaporation_rate, evaporation_temp_floor, evaporation_temp_scale, \
             iteration_count_factor, iteration_count_min, base_precipitation_rate, \
             max_precipitation_fraction, temp_capacity_offset, temp_capacity_scale, \
             orographic_factor, coastal_moisture_factor, evapotranspiration_rate, \
             moisture_cap, diffusion_rate\n\
             Hydrology: lake_basin_area_threshold, river_flow_threshold, \
             major_river_flow_threshold, floodplain_elevation_tolerance, \
             canyon_flow_slope_threshold, canyon_depth\n\
             Biomes: hydrology_override_strength, smoothing_passes, \
             forest_noise_frequency, forest_noise_amplitude"
        ),
    }
    Ok(())
}
