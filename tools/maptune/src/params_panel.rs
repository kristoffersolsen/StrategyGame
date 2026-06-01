//! Scrollable parameter form for tectonics, climate, hydrology, biomes,
//! and resource tuning parameters.

use eframe::egui;
use worldgen::{
    CFL_MAX_WIND_AMPLITUDE, MAX_DETAIL_NOISE_OCTAVES, MAX_LAND_LEVEL_COUNT, MAX_TARGET_PLATE_COUNT,
    MIN_DETAIL_NOISE_OCTAVES, MIN_LAND_LEVEL_COUNT, MIN_TARGET_PLATE_COUNT, MapSize, TuningParams,
};

/// Draws the full parameter form into `ui`.
///
/// Mutates `params`, `seed`, `map_size`, and `hex_size` directly.
pub(crate) fn show(
    ui: &mut egui::Ui,
    params: &mut TuningParams,
    seed: &mut u64,
    map_size: &mut MapSize,
    hex_size: &mut u32,
) {
    egui::ScrollArea::vertical()
        .id_salt("params_scroll")
        .show(ui, |ui| {
            global_section(ui, seed, map_size, hex_size);
            ui.separator();
            tectonics_section(ui, params);
            ui.separator();
            climate_section(ui, params);
            ui.separator();
            hydrology_section(ui, params);
            ui.separator();
            biomes_section(ui, params);
            ui.separator();
            resources_section(ui, params);
        });
}

// ------------- Section helpers -------------

fn global_section(ui: &mut egui::Ui, seed: &mut u64, map_size: &mut MapSize, hex_size: &mut u32) {
    egui::CollapsingHeader::new("Global")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Seed");
                ui.add(egui::DragValue::new(seed));
            });
            ui.horizontal(|ui| {
                ui.label("Map size");
                egui::ComboBox::from_id_salt("map_size")
                    .selected_text(map_size_label(*map_size))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(map_size, MapSize::Small, "Small (120×80)");
                        ui.selectable_value(map_size, MapSize::Standard, "Standard (180×120)");
                        ui.selectable_value(map_size, MapSize::Medium, "Medium (240×150)");
                        ui.selectable_value(map_size, MapSize::Large, "Large (320×200)");
                        ui.selectable_value(map_size, MapSize::Huge, "Huge (400×250)");
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Hex size (px)");
                ui.add(egui::DragValue::new(hex_size).range(2..=32));
            });
        });
}

fn map_size_label(s: MapSize) -> &'static str {
    match s {
        MapSize::Small => "Small",
        MapSize::Standard => "Standard",
        MapSize::Medium => "Medium",
        MapSize::Large => "Large",
        MapSize::Huge => "Huge",
        MapSize::Custom { .. } => "Custom",
    }
}

#[allow(clippy::too_many_lines)]
fn tectonics_section(ui: &mut egui::Ui, params: &mut TuningParams) {
    let defaults = worldgen::TuningParams::defaults();
    let t = &mut params.tectonics;
    let dt = &defaults.tectonics;

    egui::CollapsingHeader::new("Tectonics")
        .default_open(false)
        .show(ui, |ui| {
            u8_row(
                ui,
                "target_plate_count",
                &mut t.target_plate_count,
                dt.target_plate_count,
                MIN_TARGET_PLATE_COUNT..=MAX_TARGET_PLATE_COUNT,
            );
            u8_row(
                ui,
                "land_level_count",
                &mut t.land_level_count,
                dt.land_level_count,
                MIN_LAND_LEVEL_COUNT..=MAX_LAND_LEVEL_COUNT,
            );
            f32_row(
                ui,
                "motion_magnitude_min",
                &mut t.motion_magnitude_min,
                dt.motion_magnitude_min,
                0.01..=5.0,
            );
            f32_row(
                ui,
                "motion_magnitude_max",
                &mut t.motion_magnitude_max,
                dt.motion_magnitude_max,
                0.01..=5.0,
            );
            f32_row(
                ui,
                "plate_growth_rate_range",
                &mut t.plate_growth_rate_range,
                dt.plate_growth_rate_range,
                1.0..=5.0,
            );
            f32_row(
                ui,
                "clumping_index",
                &mut t.clumping_index,
                dt.clumping_index,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "convergence_threshold",
                &mut t.convergence_threshold,
                dt.convergence_threshold,
                0.0..=3.0,
            );
            f32_row(
                ui,
                "continental_base_elevation",
                &mut t.continental_base_elevation,
                dt.continental_base_elevation,
                -2.0..=2.0,
            );
            f32_row(
                ui,
                "oceanic_base_elevation",
                &mut t.oceanic_base_elevation,
                dt.oceanic_base_elevation,
                -2.0..=2.0,
            );
            f32_row(
                ui,
                "intra_plate_noise_amplitude",
                &mut t.intra_plate_noise_amplitude,
                dt.intra_plate_noise_amplitude,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "intra_plate_noise_frequency",
                &mut t.intra_plate_noise_frequency,
                dt.intra_plate_noise_frequency,
                0.1..=10.0,
            );
            f32_row(
                ui,
                "boundary_falloff_sigma",
                &mut t.boundary_falloff_sigma,
                dt.boundary_falloff_sigma,
                1.0..=32.0,
            );
            f32_row(
                ui,
                "boundary_strength_noise_amplitude",
                &mut t.boundary_strength_noise_amplitude,
                dt.boundary_strength_noise_amplitude,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "boundary_strength_noise_frequency",
                &mut t.boundary_strength_noise_frequency,
                dt.boundary_strength_noise_frequency,
                0.1..=10.0,
            );
            f32_row(
                ui,
                "boundary_sigma_noise_amplitude",
                &mut t.boundary_sigma_noise_amplitude,
                dt.boundary_sigma_noise_amplitude,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "boundary_sigma_noise_frequency",
                &mut t.boundary_sigma_noise_frequency,
                dt.boundary_sigma_noise_frequency,
                0.1..=10.0,
            );
            f32_row(
                ui,
                "mountain_elevation_boost",
                &mut t.mountain_elevation_boost,
                dt.mountain_elevation_boost,
                0.0..=3.0,
            );
            f32_row(
                ui,
                "coastal_mountain_boost",
                &mut t.coastal_mountain_boost,
                dt.coastal_mountain_boost,
                0.0..=3.0,
            );
            f32_row(
                ui,
                "subduction_trench_penalty",
                &mut t.subduction_trench_penalty,
                dt.subduction_trench_penalty,
                -3.0..=0.0,
            );
            f32_row(
                ui,
                "island_arc_boost",
                &mut t.island_arc_boost,
                dt.island_arc_boost,
                0.0..=2.0,
            );
            f32_row(
                ui,
                "rift_valley_penalty",
                &mut t.rift_valley_penalty,
                dt.rift_valley_penalty,
                -2.0..=0.0,
            );
            f32_row(
                ui,
                "mid_ocean_ridge_boost",
                &mut t.mid_ocean_ridge_boost,
                dt.mid_ocean_ridge_boost,
                0.0..=2.0,
            );
            f32_row(
                ui,
                "detail_noise_amplitude",
                &mut t.detail_noise_amplitude,
                dt.detail_noise_amplitude,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "detail_noise_frequency",
                &mut t.detail_noise_frequency,
                dt.detail_noise_frequency,
                0.1..=10.0,
            );
            u8_row(
                ui,
                "detail_noise_octaves",
                &mut t.detail_noise_octaves,
                dt.detail_noise_octaves,
                MIN_DETAIL_NOISE_OCTAVES..=MAX_DETAIL_NOISE_OCTAVES,
            );
            u32_row(
                ui,
                "hill_influence_radius",
                &mut t.hill_influence_radius,
                dt.hill_influence_radius,
                1..=32,
            );
            f32_row(
                ui,
                "hill_near_boundary_threshold",
                &mut t.hill_near_boundary_threshold,
                dt.hill_near_boundary_threshold,
                -1.0..=1.0,
            );
            f32_row(
                ui,
                "hill_interior_threshold",
                &mut t.hill_interior_threshold,
                dt.hill_interior_threshold,
                -1.0..=1.0,
            );
            f32_row(
                ui,
                "hill_noise_frequency",
                &mut t.hill_noise_frequency,
                dt.hill_noise_frequency,
                0.1..=10.0,
            );
            f32_row(
                ui,
                "mountain_pass_threshold",
                &mut t.mountain_pass_threshold,
                dt.mountain_pass_threshold,
                -1.0..=2.0,
            );
            f32_row(
                ui,
                "mountain_pass_noise_frequency",
                &mut t.mountain_pass_noise_frequency,
                dt.mountain_pass_noise_frequency,
                0.1..=10.0,
            );
            bool_row(
                ui,
                "mountain_pass_neighbor_aware",
                &mut t.mountain_pass_neighbor_aware,
                dt.mountain_pass_neighbor_aware,
            );
            u8_row(
                ui,
                "mountain_pass_min_mountain_neighbors",
                &mut t.mountain_pass_min_mountain_neighbors,
                dt.mountain_pass_min_mountain_neighbors,
                0..=6,
            );

            ui.label("level_proportions");
            vec_f32_row(ui, &mut t.level_proportions);
            ui.label("mineral_weights");
            vec_f32_row(ui, &mut t.mineral_weights);
        });
}

#[allow(clippy::too_many_lines)]
fn climate_section(ui: &mut egui::Ui, params: &mut TuningParams) {
    let defaults = worldgen::TuningParams::defaults();
    let c = &mut params.climate;
    let dc = &defaults.climate;

    egui::CollapsingHeader::new("Climate")
        .default_open(false)
        .show(ui, |ui| {
            f32_row(
                ui,
                "equator_temperature",
                &mut c.equator_temperature,
                dc.equator_temperature,
                -2.0..=2.0,
            );
            f32_row(
                ui,
                "pole_temperature",
                &mut c.pole_temperature,
                dc.pole_temperature,
                -2.0..=2.0,
            );
            f32_row(
                ui,
                "elevation_lapse_rate",
                &mut c.elevation_lapse_rate,
                dc.elevation_lapse_rate,
                0.0..=2.0,
            );
            f32_row(
                ui,
                "climate_band_offset",
                &mut c.climate_band_offset,
                dc.climate_band_offset,
                -1.0..=1.0,
            );
            f32_row(
                ui,
                "wind_amplitude",
                &mut c.wind_amplitude,
                dc.wind_amplitude,
                0.0..=CFL_MAX_WIND_AMPLITUDE,
            );
            f32_row(
                ui,
                "wind_noise_amplitude",
                &mut c.wind_noise_amplitude,
                dc.wind_noise_amplitude,
                0.0..=0.2,
            );
            f32_row(
                ui,
                "wind_noise_frequency",
                &mut c.wind_noise_frequency,
                dc.wind_noise_frequency,
                0.1..=5.0,
            );
            f32_row(
                ui,
                "evaporation_rate",
                &mut c.evaporation_rate,
                dc.evaporation_rate,
                0.0..=0.5,
            );
            f32_row(
                ui,
                "evaporation_temp_floor",
                &mut c.evaporation_temp_floor,
                dc.evaporation_temp_floor,
                -1.0..=1.0,
            );
            f32_row(
                ui,
                "evaporation_temp_scale",
                &mut c.evaporation_temp_scale,
                dc.evaporation_temp_scale,
                0.0..=5.0,
            );
            f32_row(
                ui,
                "base_precipitation_rate",
                &mut c.base_precipitation_rate,
                dc.base_precipitation_rate,
                0.0..=0.5,
            );
            f32_row(
                ui,
                "max_precipitation_fraction",
                &mut c.max_precipitation_fraction,
                dc.max_precipitation_fraction,
                0.01..=1.0,
            );
            f32_row(
                ui,
                "temp_capacity_offset",
                &mut c.temp_capacity_offset,
                dc.temp_capacity_offset,
                -1.0..=2.0,
            );
            f32_row(
                ui,
                "temp_capacity_scale",
                &mut c.temp_capacity_scale,
                dc.temp_capacity_scale,
                0.1..=10.0,
            );
            f32_row(
                ui,
                "orographic_factor",
                &mut c.orographic_factor,
                dc.orographic_factor,
                0.0..=5.0,
            );
            f32_row(
                ui,
                "coastal_moisture_factor",
                &mut c.coastal_moisture_factor,
                dc.coastal_moisture_factor,
                0.0..=0.2,
            );
            f32_row(
                ui,
                "evapotranspiration_rate",
                &mut c.evapotranspiration_rate,
                dc.evapotranspiration_rate,
                0.0..=0.5,
            );
            f32_row(
                ui,
                "diffusion_rate",
                &mut c.diffusion_rate,
                dc.diffusion_rate,
                0.0..=0.5,
            );
            f32_row(
                ui,
                "iteration_count_factor",
                &mut c.iteration_count_factor,
                dc.iteration_count_factor,
                0.1..=5.0,
            );
            u32_row(
                ui,
                "iteration_count_min",
                &mut c.iteration_count_min,
                dc.iteration_count_min,
                1..=1000,
            );

            ui.horizontal(|ui| {
                ui.label("moisture_cap");
                ui.add(
                    egui::DragValue::new(&mut c.moisture_cap)
                        .range(0.001..=f32::MAX)
                        .speed(0.01),
                );
                if ui.small_button("↺").clicked() {
                    c.moisture_cap = dc.moisture_cap;
                }
            });
        });
}

fn hydrology_section(ui: &mut egui::Ui, params: &mut TuningParams) {
    let defaults = worldgen::TuningParams::defaults();
    let h = &mut params.hydrology;
    let dh = &defaults.hydrology;

    egui::CollapsingHeader::new("Hydrology")
        .default_open(false)
        .show(ui, |ui| {
            u32_row(
                ui,
                "lake_basin_area_threshold",
                &mut h.lake_basin_area_threshold,
                dh.lake_basin_area_threshold,
                1..=500,
            );
            f32_row(
                ui,
                "river_flow_threshold",
                &mut h.river_flow_threshold,
                dh.river_flow_threshold,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "major_river_flow_threshold",
                &mut h.major_river_flow_threshold,
                dh.major_river_flow_threshold,
                0.0..=1.0,
            );
            f32_row(
                ui,
                "floodplain_elevation_tolerance",
                &mut h.floodplain_elevation_tolerance,
                dh.floodplain_elevation_tolerance,
                0.0..=0.5,
            );
            f32_row(
                ui,
                "canyon_flow_slope_threshold",
                &mut h.canyon_flow_slope_threshold,
                dh.canyon_flow_slope_threshold,
                0.0..=2.0,
            );
            u8_row(
                ui,
                "canyon_depth",
                &mut h.canyon_depth,
                dh.canyon_depth,
                1..=2,
            );
        });
}

fn biomes_section(ui: &mut egui::Ui, params: &mut TuningParams) {
    let defaults = worldgen::TuningParams::defaults();
    let b = &mut params.biomes;
    let db = &defaults.biomes;

    egui::CollapsingHeader::new("Biomes")
        .default_open(false)
        .show(ui, |ui| {
            f32_row(
                ui,
                "hydrology_override_strength",
                &mut b.hydrology_override_strength,
                db.hydrology_override_strength,
                0.0..=1.0,
            );
            u8_row(
                ui,
                "smoothing_passes",
                &mut b.smoothing_passes,
                db.smoothing_passes,
                0..=5,
            );
            f32_row(
                ui,
                "forest_noise_frequency",
                &mut b.forest_noise_frequency,
                db.forest_noise_frequency,
                0.05..=2.0,
            );
            f32_row(
                ui,
                "forest_noise_amplitude",
                &mut b.forest_noise_amplitude,
                db.forest_noise_amplitude,
                0.0..=1.0,
            );
        });
}

fn resources_section(ui: &mut egui::Ui, params: &mut TuningParams) {
    let defaults = worldgen::TuningParams::defaults();
    let r = &mut params.resources;
    let dr = &defaults.resources;

    egui::CollapsingHeader::new("Resources")
        .default_open(false)
        .show(ui, |ui| {
            u8_row(
                ui,
                "cluster_expansion_attempts",
                &mut r.cluster_expansion_attempts,
                dr.cluster_expansion_attempts,
                1..=32,
            );
            u8_row(
                ui,
                "poisson_max_attempts",
                &mut r.poisson_max_attempts,
                dr.poisson_max_attempts,
                5..=100,
            );
        });
}

// ------------- Row widgets -------------

fn f32_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    default: f32,
    range: std::ops::RangeInclusive<f32>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(0.001)
                .max_decimals(4),
        );
        if ui.small_button("↺").clicked() {
            *value = default;
        }
    });
}

fn u8_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u8,
    default: u8,
    range: std::ops::RangeInclusive<u8>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(value).range(range));
        if ui.small_button("↺").clicked() {
            *value = default;
        }
    });
}

fn u32_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    default: u32,
    range: std::ops::RangeInclusive<u32>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(value).range(range));
        if ui.small_button("↺").clicked() {
            *value = default;
        }
    });
}

fn bool_row(ui: &mut egui::Ui, label: &str, value: &mut bool, default: bool) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.checkbox(value, "");
        if ui.small_button("↺").clicked() {
            *value = default;
        }
    });
}

fn vec_f32_row(ui: &mut egui::Ui, values: &mut [f32]) {
    ui.horizontal_wrapped(|ui| {
        for v in values.iter_mut() {
            ui.add(
                egui::DragValue::new(v)
                    .range(0.0..=1.0)
                    .speed(0.001)
                    .max_decimals(3),
            );
        }
    });
}
