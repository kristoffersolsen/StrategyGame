//! Climate simulation (Stage 3.2, design §1.10).
//!
//! Produces four output fields — temperature, moisture, wind, and
//! accumulated precipitation — by simulating moisture advection on the
//! hex grid to pseudo-steady-state under a latitude-banded wind field
//! with orographic precipitation.
//!
//! The full stage runs in seven implementation steps:
//!
//! 1. Latitude-based temperature field (elevation lapse rate).
//! 2. Wind field: smooth cosine zonal profile + low-frequency noise.
//! 3. Evaporation source field: water tiles scaled by temperature.
//! 4. Advection step kernel: donor-cell upwind on moisture.
//! 5. Precipitation step kernel: orographic + temperature-capacity.
//! 6. Iteration driver: run steps 4–5 for a fixed count.
//! 7. Wire all steps and return [`ClimateData`].
//!
//! The mathematical model and CFL constraint are documented in
//! ADR-0005. Determinism is guaranteed by the fixed iteration count
//! and single-threaded `(r, q)` lexicographic loop order (ADR-0004).

mod advection;
mod evaporation;
mod iterate;
mod precipitation;
mod temperature;
mod wind;

use coords::MapBounds;
use rand_core::Rng;

use crate::params::ClimateParams;
use crate::rng::StageRng;
use crate::world_data::{ClimateData, TectonicData};

/// Runs the climate stage end-to-end and returns the populated
/// [`ClimateData`].
///
/// All seven steps run in sequence. The stage-scoped RNG is used
/// only to derive noise seeds for the wind field; all other
/// computation is deterministic given `params`.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `tectonics` — output of the tectonics stage; supplies elevation
///   (for temperature lapse rate and orographic precipitation) and
///   height levels (for identifying water source tiles).
/// * `params` — climate tuning parameters.
/// * `rng` — stage-scoped RNG derived from the master seed.
pub(crate) fn run(
    bounds: MapBounds,
    tectonics: &TectonicData,
    params: &ClimateParams,
    rng: &mut StageRng,
) -> ClimateData {
    // Step 1: latitude-based temperature field.
    let temperature = temperature::build_temperature_field(bounds, &tectonics.elevation, params);

    // Step 2: wind field (noise seeds drawn from stage RNG).
    let wind_noise_seed_x: u32 = rng.next_u32();
    let wind_noise_seed_y: u32 = rng.next_u32();
    let wind = wind::build_wind_field(bounds, params, wind_noise_seed_x, wind_noise_seed_y);

    // Step 3: moisture source fields (evaporation + coastal sea-breeze).
    let evaporation =
        evaporation::build_evaporation_field(bounds, &tectonics.height_level, &temperature, params);
    let coastal = evaporation::build_coastal_source_field(bounds, &tectonics.height_level, params);
    let moisture_source =
        crate::Field::from_fn(bounds, |coord| evaporation[coord] + coastal[coord]);

    // Steps 4–6: advection-precipitation iteration loop.
    let (moisture, precipitation) = iterate::run_iteration_loop(
        &wind,
        &moisture_source,
        &temperature,
        &tectonics.elevation,
        &tectonics.height_level,
        bounds,
        params,
    );

    ClimateData {
        temperature,
        moisture,
        wind,
        precipitation,
    }
}
