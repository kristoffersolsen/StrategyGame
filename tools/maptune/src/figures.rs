//! Registry of all available debug figure types.
//!
//! To add a new figure: add one entry to [`ALL_FIGURES`] and implement
//! the corresponding `render_*` function. Nothing else needs changing.

use image::RgbImage;
use worldgen::{
    BoundaryType, Direction, Field, RiverSize,
    debug_viz::{
        BiomeCmap, BoundaryCmap, Categorical, Colormap, Grayscale, HeightLevelCmap, ResourceCmap,
        Viridis, biome_handle_to_f32, render_field_hex, render_field_pixels, render_rgb_field_hex,
        resource_handle_to_f32,
    },
};

use crate::runner::WorldOutput;

/// Describes a single renderable debug figure.
pub(crate) struct FigureDescriptor {
    /// Stable `snake_case` identifier.
    pub(crate) id: &'static str,
    /// Human-readable label shown in the selector.
    pub(crate) label: &'static str,
    /// Renders the figure to an [`RgbImage`].
    pub(crate) render: fn(&WorldOutput) -> RgbImage,
}

/// All known figure types, in display order.
///
/// This is the single maintenance anchor: adding a figure means adding
/// one entry here plus the corresponding `render_*` function below.
pub(crate) const ALL_FIGURES: &[FigureDescriptor] = &[
    // ---------- Tectonics ----------
    FigureDescriptor {
        id: "height_levels_hex",
        label: "Height levels (hex)",
        render: render_height_levels_hex,
    },
    FigureDescriptor {
        id: "plate_map_hex",
        label: "Plate map (hex)",
        render: render_plate_map_hex,
    },
    FigureDescriptor {
        id: "boundary_types",
        label: "Boundary types",
        render: render_boundary_types,
    },
    FigureDescriptor {
        id: "elevation",
        label: "Elevation",
        render: render_elevation,
    },
    FigureDescriptor {
        id: "hills",
        label: "Hills",
        render: render_hills,
    },
    // ---------- Climate ----------
    FigureDescriptor {
        id: "temperature_hex",
        label: "Temperature (hex)",
        render: render_temperature_hex,
    },
    FigureDescriptor {
        id: "wind",
        label: "Wind magnitude",
        render: render_wind,
    },
    FigureDescriptor {
        id: "wind_ns",
        label: "Wind N-S component",
        render: render_wind_ns,
    },
    FigureDescriptor {
        id: "moisture",
        label: "Moisture",
        render: render_moisture,
    },
    FigureDescriptor {
        id: "precipitation_hex",
        label: "Precipitation (hex)",
        render: render_precipitation_hex,
    },
    FigureDescriptor {
        id: "precipitation_log",
        label: "Precipitation (log)",
        render: render_precipitation_log,
    },
    // ---------- Hydrology ----------
    FigureDescriptor {
        id: "flow_direction",
        label: "Flow direction",
        render: render_flow_direction,
    },
    FigureDescriptor {
        id: "flow_accumulation",
        label: "Flow accumulation",
        render: render_flow_accumulation,
    },
    FigureDescriptor {
        id: "lakes",
        label: "Lakes",
        render: render_lakes,
    },
    FigureDescriptor {
        id: "rivers",
        label: "Rivers",
        render: render_rivers,
    },
    FigureDescriptor {
        id: "floodplains",
        label: "Floodplains",
        render: render_floodplains,
    },
    FigureDescriptor {
        id: "canyons",
        label: "Canyons",
        render: render_canyons,
    },
    FigureDescriptor {
        id: "hydrology_overlay",
        label: "Hydrology overlay",
        render: render_hydrology_overlay,
    },
    // ---------- Biomes ----------
    FigureDescriptor {
        id: "biomes",
        label: "Biomes",
        render: render_biomes,
    },
    FigureDescriptor {
        id: "forest_density",
        label: "Forest density",
        render: render_forest_density,
    },
    // ---------- Resources ----------
    FigureDescriptor {
        id: "resources",
        label: "Resources",
        render: render_resources,
    },
];

// ------------- Normalisation helpers -------------

fn linear_norm(field: &Field<f32>) -> impl Fn(f32) -> f32 {
    let min = field.iter().copied().fold(f32::INFINITY, f32::min);
    let max = field.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = max - min;
    move |v| if range > 0.0 { (v - min) / range } else { 0.5 }
}

fn max_norm(field: &Field<f32>) -> impl Fn(f32) -> f32 {
    let max = field.iter().copied().fold(0.0_f32, f32::max);
    move |v| if max > 0.0 { v / max } else { 0.0 }
}

fn log_max_norm(field: &Field<f32>) -> impl Fn(f32) -> f32 {
    let max_log = field
        .iter()
        .copied()
        .map(|v| (v + 1.0).ln())
        .fold(0.0_f32, f32::max);
    move |v| {
        if max_log > 0.0 {
            (v + 1.0).ln() / max_log
        } else {
            0.0
        }
    }
}

// ------------- Render functions: tectonics -------------

fn render_height_levels_hex(w: &WorldOutput) -> RgbImage {
    render_field_hex(
        &w.world.tectonics.height_level,
        f32::from,
        &HeightLevelCmap {
            land_level_count: w.land_level_count,
        },
        w.hex_size,
    )
}

fn render_plate_map_hex(w: &WorldOutput) -> RgbImage {
    render_field_hex(
        &w.world.tectonics.plate_id,
        f32::from,
        &Categorical,
        w.hex_size,
    )
}

fn render_boundary_types(w: &WorldOutput) -> RgbImage {
    render_field_pixels(
        &w.world.tectonics.boundary_type,
        |v| match v {
            None => 0.0,
            Some(BoundaryType::Transform) => 1.0,
            Some(BoundaryType::Divergent) => 2.0,
            Some(BoundaryType::Convergent) => 3.0,
        },
        &BoundaryCmap,
    )
}

fn render_elevation(w: &WorldOutput) -> RgbImage {
    let norm = linear_norm(&w.world.tectonics.elevation);
    render_field_pixels(&w.world.tectonics.elevation, norm, &Viridis)
}

fn render_hills(w: &WorldOutput) -> RgbImage {
    render_field_pixels(
        &w.world.tectonics.is_hilly,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
    )
}

// ------------- Render functions: climate -------------

fn render_temperature_hex(w: &WorldOutput) -> RgbImage {
    let norm = linear_norm(&w.world.climate.temperature);
    render_field_hex(&w.world.climate.temperature, norm, &Viridis, w.hex_size)
}

fn render_wind(w: &WorldOutput) -> RgbImage {
    let max_mag = w
        .world
        .climate
        .wind
        .iter()
        .map(|[wx, wy]| (wx * wx + wy * wy).sqrt())
        .fold(0.0_f32, f32::max);
    render_field_pixels(
        &w.world.climate.wind,
        move |[wx, wy]: [f32; 2]| {
            if max_mag > 0.0 {
                (wx * wx + wy * wy).sqrt() / max_mag
            } else {
                0.0
            }
        },
        &Viridis,
    )
}

fn render_wind_ns(w: &WorldOutput) -> RgbImage {
    let max_ns = w
        .world
        .climate
        .wind
        .iter()
        .map(|v| v[1].abs())
        .fold(0.0_f32, f32::max);
    render_field_pixels(
        &w.world.climate.wind,
        move |[_wx, wy]: [f32; 2]| {
            if max_ns > 0.0 {
                wy / max_ns * 0.5 + 0.5
            } else {
                0.5
            }
        },
        &Viridis,
    )
}

fn render_moisture(w: &WorldOutput) -> RgbImage {
    let norm = max_norm(&w.world.climate.moisture);
    render_field_pixels(&w.world.climate.moisture, norm, &Viridis)
}

fn render_precipitation_hex(w: &WorldOutput) -> RgbImage {
    let norm = max_norm(&w.world.climate.precipitation);
    render_field_hex(&w.world.climate.precipitation, norm, &Viridis, w.hex_size)
}

fn render_precipitation_log(w: &WorldOutput) -> RgbImage {
    let norm = log_max_norm(&w.world.climate.precipitation);
    render_field_hex(&w.world.climate.precipitation, norm, &Viridis, w.hex_size)
}

// ------------- Render functions: hydrology -------------

fn render_flow_direction(w: &WorldOutput) -> RgbImage {
    render_field_hex(
        &w.world.hydrology.flow_direction,
        flow_direction_to_f32,
        &Categorical,
        w.hex_size,
    )
}

fn render_flow_accumulation(w: &WorldOutput) -> RgbImage {
    let norm = log_max_norm(&w.world.hydrology.flow_accumulation);
    render_field_hex(
        &w.world.hydrology.flow_accumulation,
        norm,
        &Viridis,
        w.hex_size,
    )
}

fn render_lakes(w: &WorldOutput) -> RgbImage {
    render_field_hex(
        &w.world.hydrology.height_level,
        |level| {
            if level == 1 { 0.0 } else { f32::from(level) }
        },
        &HeightLevelCmap {
            land_level_count: w.land_level_count,
        },
        w.hex_size,
    )
}

fn render_rivers(w: &WorldOutput) -> RgbImage {
    render_field_hex(
        &w.world.hydrology.river_size,
        river_size_to_f32,
        &Categorical,
        w.hex_size,
    )
}

fn render_floodplains(w: &WorldOutput) -> RgbImage {
    render_field_pixels(
        &w.world.hydrology.is_floodplain,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
    )
}

fn render_canyons(w: &WorldOutput) -> RgbImage {
    let canyon_overlay = Field::from_fn(w.world.hydrology.is_canyon.bounds(), |coord| {
        if w.world.hydrology.is_canyon[coord] {
            2_u8
        } else {
            u8::from(w.world.hydrology.is_canyon_wall[coord])
        }
    });
    render_field_pixels(&canyon_overlay, f32::from, &Categorical)
}

fn render_hydrology_overlay(w: &WorldOutput) -> RgbImage {
    let height_cmap = HeightLevelCmap {
        land_level_count: w.land_level_count,
    };
    let overlay = Field::from_fn(w.world.tectonics.height_level.bounds(), |coord| {
        if w.world.hydrology.is_lake[coord] {
            [0_u8, 170, 170]
        } else if w.world.hydrology.is_canyon[coord] || w.world.hydrology.is_canyon_wall[coord] {
            [140_u8, 85, 35]
        } else if w.world.hydrology.river_size[coord] == Some(RiverSize::MajorRiver) {
            [130_u8, 70, 210]
        } else if w.world.hydrology.river_size[coord].is_some() {
            [255_u8, 105, 180]
        } else {
            height_cmap.color(f32::from(w.world.hydrology.height_level[coord]))
        }
    });
    render_rgb_field_hex(&overlay, w.hex_size)
}

// ------------- Render functions: biomes -------------

fn render_biomes(w: &WorldOutput) -> RgbImage {
    let cmap = BiomeCmap::new(&w.world.biome_registry);
    render_field_hex(
        &w.world.biomes.biome,
        biome_handle_to_f32,
        &cmap,
        w.hex_size,
    )
}

fn render_forest_density(w: &WorldOutput) -> RgbImage {
    render_field_pixels(&w.world.biomes.forest_density, |v| v, &Viridis)
}

// ------------- Render functions: resources -------------

fn render_resources(w: &WorldOutput) -> RgbImage {
    let cmap = ResourceCmap::new(&w.world.resource_registry);
    render_field_hex(
        &w.world.resources.resource,
        resource_handle_to_f32,
        &cmap,
        w.hex_size,
    )
}

// ------------- Enum-to-f32 helpers -------------

fn flow_direction_to_f32(direction: Option<Direction>) -> f32 {
    match direction {
        None => 0.0,
        Some(Direction::East) => 1.0,
        Some(Direction::SouthEast) => 2.0,
        Some(Direction::SouthWest) => 3.0,
        Some(Direction::West) => 4.0,
        Some(Direction::NorthWest) => 5.0,
        Some(Direction::NorthEast) => 6.0,
    }
}

fn river_size_to_f32(river_size: Option<RiverSize>) -> f32 {
    match river_size {
        None => 0.0,
        Some(RiverSize::Creek) => 1.0,
        Some(RiverSize::River) => 2.0,
        Some(RiverSize::MajorRiver) => 3.0,
    }
}
