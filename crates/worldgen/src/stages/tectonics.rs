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
//!
//! Each step's results are populated into [`TectonicData`] (in
//! [`crate::world_data`]). Step 1 produces the `seeds` field; steps
//! 2–3 produce `plate_id`; later steps populate the remaining fields.
//!
//! The entry point [`run`] is called by [`crate::generate_world`] and
//! returns the populated tectonic data for the world.

mod perturbation;
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
/// Steps implemented so far:
/// - Step 1: plate seed placement (Poisson disk) → `seeds`.
/// - Steps 2–3: noise-perturbed Voronoi partition → `plate_id`.
///
/// # Errors
///
/// Returns [`GenerationError`] if any sub-step fails. Steps 1–3 are
/// currently infallible; the `Result` return type is in place so
/// step 6 (base elevation) can surface numerical failures without a
/// signature change.
// Steps 4–9 will fill in the remaining TectonicData fields; the
// Result return type is pre-declared to avoid a signature change.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run(
    bounds: MapBounds,
    params: &TectonicsParams,
    rng: &mut StageRng,
) -> Result<TectonicData, GenerationError> {
    // Step 1: Poisson-disk plate seed placement.
    let seeds = place_plate_seeds(bounds, params, rng);

    // Steps 2–3: noise-perturbed Voronoi partition.
    // Draw a u32 from the stage RNG to seed the noise function
    // independently from the seed-placement RNG stream.
    let noise_seed: u32 = rng.next_u32();
    let plate_id = perturbation::assign_plates_perturbed(
        bounds,
        &seeds,
        params.boundary_perturbation_amplitude,
        noise_seed,
    );

    Ok(TectonicData { seeds, plate_id })
}
