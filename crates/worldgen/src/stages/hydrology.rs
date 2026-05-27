//! Hydrology generation (Stage 3.3, design §1.11).
//!
//! Produces river networks, lakes, floodplains, canyons, canyon walls,
//! and the temporary post-hydrology height-level field.

mod accumulation;
mod basin;
mod canyon;
mod floodplain;
mod flow_direction;
mod rivers;

use coords::MapBounds;

use crate::params::HydrologyParams;
use crate::world_data::{ClimateData, HydrologyData, TectonicData};

/// Runs the hydrology stage end-to-end and returns the populated
/// [`HydrologyData`].
pub(crate) fn run(
    bounds: MapBounds,
    tectonics: &TectonicData,
    climate: &ClimateData,
    params: &HydrologyParams,
) -> HydrologyData {
    // Step 1: initial D6 flow direction over tectonic elevation.
    let initial_flow = flow_direction::build_flow_direction(
        bounds,
        &tectonics.elevation,
        &tectonics.height_level,
        &crate::Field::filled(bounds, false),
    );

    // Steps 2–3: detect sinks and resolve them by filling or lake marking.
    let basins = basin::detect_basins(
        bounds,
        &tectonics.elevation,
        &initial_flow,
        &tectonics.height_level,
    );
    let basin::ResolvedBasins {
        elevation,
        height_level,
        is_lake,
        lake_outlet,
    } = basin::resolve_basins(
        bounds,
        &tectonics.elevation,
        &tectonics.height_level,
        &basins,
        params,
    );

    let flow_direction =
        flow_direction::build_flow_direction(bounds, &elevation, &height_level, &is_lake);

    // Step 4: precipitation-weighted flow accumulation.
    let flow_accumulation = accumulation::accumulate_flow(
        bounds,
        &flow_direction,
        &elevation,
        &height_level,
        &is_lake,
        &lake_outlet,
        &climate.precipitation,
    );

    // Step 5: river tier classification.
    let river_size = rivers::classify_rivers(&flow_accumulation, &height_level, &is_lake, params);

    // Step 6: floodplain feature marking.
    let is_floodplain =
        floodplain::mark_floodplains(bounds, &river_size, &elevation, &height_level, params);

    // Step 7: post-hoc canyon carving.
    let canyon::CanyonResult {
        height_level,
        is_canyon,
        is_canyon_wall,
    } = canyon::carve_canyons(
        bounds,
        &flow_direction,
        &flow_accumulation,
        &river_size,
        &elevation,
        &height_level,
        params,
    );

    HydrologyData {
        flow_direction,
        flow_accumulation,
        is_lake,
        lake_outlet,
        river_size,
        is_floodplain,
        is_canyon,
        is_canyon_wall,
        height_level,
    }
}
