//! Sets a single scalar [`TuningParams`] field by `snake_case` name.
//!
//! Mirrors `tools/maptest/src/sweep.rs`. Both must be kept in sync when
//! new parameters are added to `TuningParams`.

use anyhow::Result;
use worldgen::{BiomeParams, ResourceParams, TectonicsParams, TuningParams};

/// Sweepable scalar parameter names, in the order they appear in the
/// dropdown. Vec and bool fields are excluded.
pub(crate) const SWEEPABLE_PARAMS: &[&str] = &[
    // Tectonics
    "target_plate_count",
    "land_level_count",
    "detail_noise_octaves",
    "hill_influence_radius",
    "motion_magnitude_min",
    "motion_magnitude_max",
    "plate_growth_rate_range",
    "clumping_index",
    "convergence_threshold",
    "continental_base_elevation",
    "oceanic_base_elevation",
    "intra_plate_noise_amplitude",
    "intra_plate_noise_frequency",
    "boundary_falloff_sigma",
    "boundary_strength_noise_amplitude",
    "boundary_strength_noise_frequency",
    "boundary_sigma_noise_amplitude",
    "boundary_sigma_noise_frequency",
    "mountain_elevation_boost",
    "coastal_mountain_boost",
    "subduction_trench_penalty",
    "island_arc_boost",
    "rift_valley_penalty",
    "mid_ocean_ridge_boost",
    "detail_noise_amplitude",
    "detail_noise_frequency",
    "hill_near_boundary_threshold",
    "hill_interior_threshold",
    "hill_noise_frequency",
    "mountain_pass_threshold",
    "mountain_pass_noise_frequency",
    "mountain_pass_min_mountain_neighbors",
    // Climate
    "equator_temperature",
    "pole_temperature",
    "elevation_lapse_rate",
    "climate_band_offset",
    "wind_amplitude",
    "wind_noise_amplitude",
    "wind_noise_frequency",
    "evaporation_rate",
    "evaporation_temp_floor",
    "evaporation_temp_scale",
    "iteration_count_factor",
    "iteration_count_min",
    "base_precipitation_rate",
    "max_precipitation_fraction",
    "temp_capacity_offset",
    "temp_capacity_scale",
    "orographic_factor",
    "coastal_moisture_factor",
    "evapotranspiration_rate",
    "moisture_cap",
    "diffusion_rate",
    // Hydrology
    "lake_basin_area_threshold",
    "river_flow_threshold",
    "major_river_flow_threshold",
    "floodplain_elevation_tolerance",
    "canyon_flow_slope_threshold",
    "canyon_depth",
    // Biomes
    "hydrology_override_strength",
    "smoothing_passes",
    "forest_noise_frequency",
    "forest_noise_amplitude",
    // Resources
    "cluster_expansion_attempts",
    "poisson_max_attempts",
];

/// Sets a single scalar field of `tuning` by its `snake_case` name.
///
/// # Errors
///
/// Returns an error if `name` is unknown or is a non-scalar (Vec/bool) field.
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
    if set_resource_param(&mut tuning.resources, name, value) {
        return Ok(());
    }

    let c = &mut tuning.climate;
    let h = &mut tuning.hydrology;
    match name {
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
        "lake_basin_area_threshold" => h.lake_basin_area_threshold = value.round() as u32,
        "river_flow_threshold" => h.river_flow_threshold = value as f32,
        "major_river_flow_threshold" => h.major_river_flow_threshold = value as f32,
        "floodplain_elevation_tolerance" => h.floodplain_elevation_tolerance = value as f32,
        "canyon_flow_slope_threshold" => h.canyon_flow_slope_threshold = value as f32,
        "canyon_depth" => h.canyon_depth = value.round() as u8,
        _ => anyhow::bail!("unknown TuningParams field: `{name}`"),
    }
    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn set_tectonics_param(t: &mut TectonicsParams, name: &str, value: f64) -> Result<bool> {
    match name {
        "target_plate_count" => t.target_plate_count = value.round() as u8,
        "land_level_count" => t.land_level_count = value.round() as u8,
        "detail_noise_octaves" => t.detail_noise_octaves = value.round() as u8,
        "hill_influence_radius" => t.hill_influence_radius = value.round() as u32,
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
        "mountain_pass_min_mountain_neighbors" => {
            t.mountain_pass_min_mountain_neighbors = value.round().clamp(0.0, 6.0) as u8;
        }
        "level_proportions" | "mineral_weights" => {
            anyhow::bail!("`{name}` is a Vec field; edit it via RON file")
        }
        "mountain_pass_neighbor_aware" => {
            anyhow::bail!("`{name}` is a bool field; edit it via RON file")
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
        "temperature_bin_count" | "moisture_bin_count" => {
            anyhow::bail!("`{name}` is coupled to the biome table; edit table and params together")
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
fn set_resource_param(resources: &mut ResourceParams, name: &str, value: f64) -> bool {
    match name {
        "cluster_expansion_attempts" => resources.cluster_expansion_attempts = value.round() as u8,
        "poisson_max_attempts" => resources.poisson_max_attempts = value.round() as u8,
        _ => return false,
    }
    true
}
