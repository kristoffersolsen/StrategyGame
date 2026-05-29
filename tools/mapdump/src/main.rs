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
//! mapdump --seed 42 --output /tmp/map-debug/ --params presets/default.ron
//! mapdump --seed 42 --output /tmp/map-debug/ --size large --hex-size 6
//! ```
//!
//! # Parameter overrides
//!
//! Pass `--params <file.ron>` to load a [`TuningParams`] from a RON
//! file instead of using defaults. Use the `maptest` tool for multi-seed
//! runs and parameter-range sweeps.
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
//! ## Stage 3.1 — Tectonics
//!
//! - `31_01_plate_seeds_px.png` — seed placement validation.
//! - `31_03_plate_map_px.png` / `31_03_plate_map_hex.png` — plate map.
//! - `31_05_boundary_types_px.png` — convergent/divergent/transform.
//! - `31_07_elevation_px.png` — normalized elevation field.
//! - `31_08_height_levels_px.png` / `31_08_height_levels_hex.png` — discrete levels.
//! - `31_09_hills_px.png` — hilliness field.
//!
//! ## Stage 3.2 — Climate
//!
//! - `32_01_temperature_px.png` / `32_01_temperature_hex.png` — temperature field.
//! - `32_02_wind_px.png` — wind magnitude field.
//! - `32_06_moisture_px.png` — final steady-state moisture field.
//! - `32_06_precipitation_px.png` / `32_06_precipitation_hex.png` — total precipitation.
//!
//! ## Stage 3.3 — Hydrology
//!
//! - `33_01_flow_direction_hex.png` — D6 downstream direction field.
//! - `33_02_flow_accumulation_hex.png` — log-scale accumulated flow.
//! - `33_03_lakes_hex.png` — lake tiles.
//! - `33_04_rivers_hex.png` — river tiers.
//! - `33_05_floodplains_px.png` — floodplain feature mask.
//! - `33_06_canyons_px.png` — canyon floor and wall mask.
//! - `33_07_hydrology_overlay_hex.png` — elevation with lakes, rivers, and canyons.
//!
//! ## Stage 3.4 — Biomes
//!
//! - `34_01_biomes_hex.png` — categorical biome map.
//! - `34_06_forest_density_px.png` — forest-density feature field.
//!
//! ## Stage 3.5 — Resources
//!
//! - `35_01_resources_hex.png` — categorical combined resource map.
//! - `35_02_resource_<name>_px.png` — binary overlay for each resource.

use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tracing::info;
use worldgen::{
    BasicParams, BiomeData, BoundaryType, ClimateData, Direction, Field, HydrologyData, MapSize,
    ResourceData, RiverSize, TectonicData, TuningParams, WorldData, WorldParams,
    debug_viz::{
        BiomeCmap, BoundaryCmap, Categorical, Colormap, Grayscale, HeightLevelCmap, ResourceCmap,
        Viridis, biome_handle_to_f32, dump_field_hex, dump_field_pixels,
        dump_field_with_seed_overlay, dump_rgb_field_hex, resource_handle_to_f32,
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

    let tuning = load_tuning_params(args.params.as_deref())?;

    let mut params = WorldParams {
        seed: args.seed,
        basic: BasicParams {
            map_size: args.size.into(),
            ..BasicParams::default()
        },
        ..WorldParams::default()
    };
    params.tuning = tuning;

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
    write_climate(&world.climate, &args.output, args.hex_size)?;
    write_hydrology(
        &world.tectonics,
        &world.hydrology,
        &params,
        &args.output,
        args.hex_size,
    )?;
    write_biomes(&world.biomes, &args.output, args.hex_size, &world)?;
    write_resources(&world.resources, &args.output, args.hex_size, &world)?;

    info!("done");
    Ok(())
}

// ------------- Parameter loading -------------

fn load_tuning_params(path: Option<&Path>) -> Result<TuningParams> {
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

// ------------- Output helpers (stage 0 and tectonics) -------------

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
    info!(plate_count, "organic plate growth complete");

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
        &HeightLevelCmap {
            land_level_count: params.tuning.tectonics.land_level_count,
        },
    )
    .with_context(|| format!("failed to write {}", levels_px_path.display()))?;

    let levels_hex_path = output.join("31_08_height_levels_hex.png");
    info!(path = %levels_hex_path.display(), hex_size, "writing height levels hex PNG");
    dump_field_hex(
        &tectonics.height_level,
        &levels_hex_path,
        f32::from,
        &HeightLevelCmap {
            land_level_count: params.tuning.tectonics.land_level_count,
        },
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

// ------------- Output helpers (climate) -------------

fn write_climate(climate: &ClimateData, output: &Path, hex_size: u32) -> Result<()> {
    // Temperature.
    let t_min = climate
        .temperature
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let t_max = climate
        .temperature
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let t_range = t_max - t_min;
    info!(min = t_min, max = t_max, "temperature field stats");

    let temp_to_f32 = move |v: f32| {
        if t_range > 0.0 {
            (v - t_min) / t_range
        } else {
            0.5
        }
    };
    let temp_px_path = output.join("32_01_temperature_px.png");
    info!(path = %temp_px_path.display(), "writing temperature pixel PNG");
    dump_field_pixels(&climate.temperature, &temp_px_path, temp_to_f32, &Viridis)
        .with_context(|| format!("failed to write {}", temp_px_path.display()))?;

    let temp_hex_path = output.join("32_01_temperature_hex.png");
    info!(path = %temp_hex_path.display(), hex_size, "writing temperature hex PNG");
    dump_field_hex(
        &climate.temperature,
        &temp_hex_path,
        temp_to_f32,
        &Viridis,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", temp_hex_path.display()))?;

    // Wind magnitude.
    let max_mag = climate
        .wind
        .iter()
        .map(|[wx, wy]| (wx * wx + wy * wy).sqrt())
        .fold(0.0_f32, f32::max);
    info!(max_magnitude = max_mag, "wind field stats");

    let wind_px_path = output.join("32_02_wind_px.png");
    info!(path = %wind_px_path.display(), "writing wind magnitude PNG");
    dump_field_pixels(
        &climate.wind,
        &wind_px_path,
        move |[wx, wy]: [f32; 2]| {
            if max_mag > 0.0 {
                (wx * wx + wy * wy).sqrt() / max_mag
            } else {
                0.0
            }
        },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", wind_px_path.display()))?;

    // Moisture.
    let max_moist = climate.moisture.iter().copied().fold(0.0_f32, f32::max);
    info!(max = max_moist, "moisture field stats");

    let moist_px_path = output.join("32_06_moisture_px.png");
    info!(path = %moist_px_path.display(), "writing moisture PNG");
    dump_field_pixels(
        &climate.moisture,
        &moist_px_path,
        move |v| if max_moist > 0.0 { v / max_moist } else { 0.0 },
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", moist_px_path.display()))?;

    // Precipitation.
    let max_precip = climate
        .precipitation
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);
    info!(max = max_precip, "precipitation field stats");

    let precip_to_f32 = move |v: f32| {
        if max_precip > 0.0 {
            v / max_precip
        } else {
            0.0
        }
    };
    let precip_px_path = output.join("32_06_precipitation_px.png");
    info!(path = %precip_px_path.display(), "writing precipitation pixel PNG");
    dump_field_pixels(
        &climate.precipitation,
        &precip_px_path,
        precip_to_f32,
        &Viridis,
    )
    .with_context(|| format!("failed to write {}", precip_px_path.display()))?;

    let precip_hex_path = output.join("32_06_precipitation_hex.png");
    info!(path = %precip_hex_path.display(), hex_size, "writing precipitation hex PNG");
    dump_field_hex(
        &climate.precipitation,
        &precip_hex_path,
        precip_to_f32,
        &Viridis,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", precip_hex_path.display()))?;

    Ok(())
}

// ------------- Output helpers (hydrology) -------------

fn write_hydrology(
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    params: &WorldParams,
    output: &Path,
    hex_size: u32,
) -> Result<()> {
    let flow_direction_path = output.join("33_01_flow_direction_hex.png");
    info!(path = %flow_direction_path.display(), "writing flow direction PNG");
    dump_field_hex(
        &hydrology.flow_direction,
        &flow_direction_path,
        flow_direction_to_f32,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", flow_direction_path.display()))?;

    let max_log = hydrology
        .flow_accumulation
        .iter()
        .copied()
        .map(|v| (v + 1.0).ln())
        .fold(0.0_f32, f32::max);
    let flow_to_f32 = move |v: f32| {
        if max_log > 0.0 {
            (v + 1.0).ln() / max_log
        } else {
            0.0
        }
    };
    let accumulation_path = output.join("33_02_flow_accumulation_hex.png");
    info!(path = %accumulation_path.display(), "writing flow accumulation PNG");
    dump_field_hex(
        &hydrology.flow_accumulation,
        &accumulation_path,
        flow_to_f32,
        &Viridis,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", accumulation_path.display()))?;

    let lakes_path = output.join("33_03_lakes_hex.png");
    info!(path = %lakes_path.display(), "writing lakes PNG");
    dump_field_hex(
        &hydrology.is_lake,
        &lakes_path,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", lakes_path.display()))?;

    let rivers_path = output.join("33_04_rivers_hex.png");
    info!(path = %rivers_path.display(), "writing rivers PNG");
    dump_field_hex(
        &hydrology.river_size,
        &rivers_path,
        river_size_to_f32,
        &Categorical,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", rivers_path.display()))?;

    let floodplains_path = output.join("33_05_floodplains_px.png");
    info!(path = %floodplains_path.display(), "writing floodplains PNG");
    dump_field_pixels(
        &hydrology.is_floodplain,
        &floodplains_path,
        |v| if v { 1.0 } else { 0.0 },
        &Grayscale,
    )
    .with_context(|| format!("failed to write {}", floodplains_path.display()))?;

    let canyon_overlay = Field::from_fn(hydrology.is_canyon.bounds(), |coord| {
        if hydrology.is_canyon[coord] {
            2_u8
        } else {
            u8::from(hydrology.is_canyon_wall[coord])
        }
    });
    let canyons_path = output.join("33_06_canyons_px.png");
    info!(path = %canyons_path.display(), "writing canyons PNG");
    dump_field_pixels(&canyon_overlay, &canyons_path, f32::from, &Categorical)
        .with_context(|| format!("failed to write {}", canyons_path.display()))?;

    let overlay = hydrology_overlay_field(
        tectonics,
        hydrology,
        params.tuning.tectonics.land_level_count,
    );
    let overlay_path = output.join("33_07_hydrology_overlay_hex.png");
    info!(path = %overlay_path.display(), "writing hydrology overlay PNG");
    dump_rgb_field_hex(&overlay, &overlay_path, hex_size)
        .with_context(|| format!("failed to write {}", overlay_path.display()))?;

    Ok(())
}

fn hydrology_overlay_field(
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    land_level_count: u8,
) -> Field<[u8; 3]> {
    let height_cmap = HeightLevelCmap { land_level_count };
    Field::from_fn(tectonics.height_level.bounds(), |coord| {
        if hydrology.is_lake[coord] {
            LAKE_COLOR
        } else if hydrology.is_canyon[coord] || hydrology.is_canyon_wall[coord] {
            CANYON_COLOR
        } else if hydrology.river_size[coord] == Some(RiverSize::MajorRiver) {
            MAJOR_RIVER_COLOR
        } else if hydrology.river_size[coord].is_some() {
            RIVER_COLOR
        } else {
            height_cmap.color(f32::from(hydrology.height_level[coord]))
        }
    })
}

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

// ------------- Output helpers (biomes) -------------

fn write_biomes(biomes: &BiomeData, output: &Path, hex_size: u32, world: &WorldData) -> Result<()> {
    let biome_path = output.join("34_01_biomes_hex.png");
    info!(path = %biome_path.display(), "writing biome PNG");
    let biome_cmap = BiomeCmap::new(&world.biome_registry);
    dump_field_hex(
        &biomes.biome,
        &biome_path,
        biome_handle_to_f32,
        &biome_cmap,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", biome_path.display()))?;

    let forest_path = output.join("34_06_forest_density_px.png");
    info!(path = %forest_path.display(), "writing forest density PNG");
    dump_field_pixels(&biomes.forest_density, &forest_path, |v| v, &Viridis)
        .with_context(|| format!("failed to write {}", forest_path.display()))?;

    Ok(())
}

// ------------- Output helpers (resources) -------------

fn write_resources(
    resources: &ResourceData,
    output: &Path,
    hex_size: u32,
    world: &WorldData,
) -> Result<()> {
    let combined_path = output.join("35_01_resources_hex.png");
    info!(path = %combined_path.display(), "writing resources PNG");
    let resource_cmap = ResourceCmap::new(&world.resource_registry);
    dump_field_hex(
        &resources.resource,
        &combined_path,
        resource_handle_to_f32,
        &resource_cmap,
        hex_size,
    )
    .with_context(|| format!("failed to write {}", combined_path.display()))?;

    for (handle, id) in world.resource_registry.inner.iter() {
        let path = output.join(format!("35_02_resource_{id}_px.png"));
        info!(path = %path.display(), resource = %id, "writing resource overlay PNG");
        dump_field_pixels(
            &resources.resource,
            &path,
            |tile| {
                if tile == Some(handle) { 1.0 } else { 0.0 }
            },
            &Grayscale,
        )
        .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

const LAKE_COLOR: [u8; 3] = [0, 170, 170];
const RIVER_COLOR: [u8; 3] = [255, 105, 180];
const MAJOR_RIVER_COLOR: [u8; 3] = [130, 70, 210];
const CANYON_COLOR: [u8; 3] = [140, 85, 35];

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

    /// Optional RON file containing a `TuningParams` struct to use
    /// instead of defaults. Use `maptest` for multi-seed runs and sweeps.
    #[arg(long)]
    params: Option<PathBuf>,
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
