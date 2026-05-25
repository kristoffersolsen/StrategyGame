//! Tectonic-plate generation (Stage 3.1, design §1.9).
//!
//! Implements the faked-tectonics pipeline: produce geological-looking
//! output without simulating geological history. The full stage runs
//! in nine implementation steps:
//!
//! 1. Plate seed placement (Poisson disk).
//! 2. Voronoi partition assigning every tile to a plate.
//! 3. Boundary perturbation (low-frequency noise on the partition).
//! 4. Plate type / motion / mineral character assignment.
//! 5. Boundary classification (convergent / divergent / transform).
//! 6. Base elevation field (plate type + intra-plate noise).
//! 7. Boundary-effect propagation via distance-field Gaussian falloff.
//! 8. Detail-noise overlay.
//! 9. Sea-level threshold and discrete height-level discretisation.
//! 10. Hills pass.
//!
//! Each step's results are populated into [`TectonicData`] (in
//! [`crate::world_data`]). The entry point [`run`] is called by
//! [`crate::generate_world`] and returns the populated tectonic data.

mod boundary;
mod elevation;
mod hills;
mod perturbation;
mod plates;
mod sea_level;
mod seeds;
pub(crate) mod voronoi;

pub(crate) use seeds::place_plate_seeds;

use coords::MapBounds;
use rand_core::Rng;

use crate::error::GenerationError;
use crate::params::TectonicsParams;
use crate::rng::StageRng;
use crate::world_data::TectonicData;

/// Runs the tectonics stage end-to-end against the given parameters
/// and stage-scoped RNG, returning the populated [`TectonicData`].
///
/// All steps 1–9 are run in sequence:
/// - Step 1: plate seed placement (Poisson disk) → `seeds`.
/// - Steps 2–3: noise-perturbed Voronoi partition → `plate_id`.
/// - Step 4: plate type, motion vector, mineral character → `plates`.
/// - Step 5: boundary classification → `boundary_type`.
/// - Steps 6–7: base elevation + detail noise → `elevation`.
/// - Step 8: sea-level correction + height-level discretisation
///   → normalised `elevation`, `height_level`.
/// - Step 9: hills pass → `is_hilly`.
///
/// # Errors
///
/// Returns [`GenerationError`] if any sub-step fails. The `Result`
/// return type is in place for future numerical failure modes (e.g.
/// degenerate elevation distributions); current steps are infallible.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run(
    bounds: MapBounds,
    params: &TectonicsParams,
    rng: &mut StageRng,
) -> Result<TectonicData, GenerationError> {
    // Step 1: Poisson-disk plate seed placement.
    let seeds = place_plate_seeds(bounds, params, rng);

    // Steps 2–3: noise-perturbed Voronoi partition.
    let voronoi_noise_seed: u32 = rng.next_u32();
    let plate_id = perturbation::assign_plates_perturbed(
        bounds,
        &seeds,
        params.boundary_perturbation_amplitude,
        voronoi_noise_seed,
    );

    // Step 4: plate type, motion vector, mineral character.
    let plates = plates::assign_plate_properties(seeds.len(), params, rng);

    // Step 5: boundary classification.
    let boundary_type = boundary::classify_boundaries(
        bounds,
        &plate_id,
        &plates,
        &seeds,
        params.convergence_threshold,
    );

    // Steps 6–7: base elevation field + detail-noise overlay.
    let intra_noise_seed: u32 = rng.next_u32();
    let detail_noise_seed: u32 = rng.next_u32();
    let mut elevation = elevation::build_elevation_field(
        bounds,
        &plate_id,
        &plates,
        &boundary_type,
        params,
        intra_noise_seed,
        detail_noise_seed,
    );

    // Step 8: sea-level correction + height-level discretisation.
    let height_level = sea_level::quantize_elevation(bounds, &mut elevation, params);

    // Step 9: hills pass.
    let hill_noise_seed: u32 = rng.next_u32();
    let is_hilly = hills::classify_hills(bounds, &boundary_type, params, hill_noise_seed);

    Ok(TectonicData {
        seeds,
        plate_id,
        plates,
        boundary_type,
        elevation,
        height_level,
        is_hilly,
    })
}
