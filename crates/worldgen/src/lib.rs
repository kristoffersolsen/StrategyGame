//! Procedural world generation pipeline.
//!
//! Transforms a seed and a [`WorldParams`] configuration into a complete
//! [`WorldData`] structure describing a generated map. The pipeline runs
//! in six stages: tectonics, climate, hydrology, biomes, resources, and
//! starting position selection.
//!
//! This crate has no dependency on Bevy and can be run standalone via the
//! `mapdump` CLI tool. All output is deterministic — the same seed and
//! parameters always produce byte-identical results.
//!
//! # Determinism
//!
//! Every generation stage that consumes randomness derives a sub-RNG
//! from a single master seed using a fixed stage identifier as salt.
//! Tuning the parameters of stage *N* therefore does not shift the
//! output of stage *M* > *N*, which would otherwise make iterative
//! tuning impossible.
//!
//! The full RNG-threading strategy is documented in
//! `docs/adr/0001-determinism-and-rng-threading.md`.
//!
//! Hard rules enforced by lints and review:
//! - No `rand::thread_rng` / `rand::rng` anywhere in the workspace.
//! - No `HashMap` iteration whose order affects generation output;
//!   use `BTreeMap`, or `sort` before iterating.
//! - No parallel reductions in inner generation loops unless an
//!   explicit deterministic reduction is used.
//!
//! # Pipeline overview
//!
//! ```text
//! WorldParams + seed
//!     │
//!     ▼
//! generate_world  ─►  WorldData
//!     │
//!     ├─ tectonics            (stage 3.1, design §1.9)
//!     ├─ climate              (stage 3.2, design §1.10)
//!     ├─ hydrology            (stage 3.3, design §1.11)
//!     ├─ biomes               (stage 3.4, design §1.12)
//!     ├─ resources            (stage 3.5, design §1.13)
//!     └─ starting positions   (stage 3.6, design §1.14)
//! ```
//!
//! Each stage takes the prior stages' output and contributes its own
//! sub-structure to [`WorldData`]. See `docs/design/overview.md`
//! §1.9–1.14 for the algorithm design.

pub mod debug_viz;
mod error;
mod field;
mod params;
pub mod rng;
mod stages;
mod world_data;

pub use coords::Direction;
pub use error::GenerationError;
pub use field::Field;
pub use params::{
    AdvancedParams, BasicParams, BiomeParams, CFL_MAX_WIND_AMPLITUDE, ClimateBand, ClimateParams,
    HydrologyParams, MAX_DETAIL_NOISE_OCTAVES, MAX_LAND_LEVEL_COUNT, MAX_TARGET_PLATE_COUNT,
    MIN_DETAIL_NOISE_OCTAVES, MIN_LAND_LEVEL_COUNT, MIN_TARGET_PLATE_COUNT, MapArchetype, MapSize,
    ResourceAbundance, ResourceParams, TectonicsParams, TuningParams, WorldAge, WorldParams,
};
pub use world_data::{
    Biome, BiomeData, BoundaryType, ClimateData, HydrologyData, MineralCharacter, MotionVector,
    Plate, PlateId, PlateType, ResourceData, ResourceId, RiverSize, TectonicData, WorldData,
};

use coords::MapBounds;
use rng::{MasterRng, StageId};

/// Runs the full world-generation pipeline against `params` and returns
/// the finalized [`WorldData`].
///
/// This is the single entry point used by both the `game` crate (at
/// startup, on a background thread) and the `mapdump` tool (headlessly,
/// for parameter tuning).
///
/// # Errors
///
/// Returns [`GenerationError`] if any pipeline stage fails. Current
/// failure modes:
/// - [`GenerationError::InvalidParams`] — the supplied [`WorldParams`]
///   are internally inconsistent (e.g. zero map dimensions, or a
///   `TectonicsParams` field outside its valid range).
///
/// Additional failure modes will be added as later stages are
/// implemented; see the variants on [`GenerationError`].
///
/// # Determinism
///
/// The returned [`WorldData`] is a deterministic function of `params`.
/// The same `params` value always produces byte-identical output,
/// across machines and across rebuilds.
pub fn generate_world(params: &WorldParams) -> Result<WorldData, GenerationError> {
    params.validate()?;

    let (width, height) = params.basic.map_size.dimensions();
    let bounds = MapBounds::new(width, height);
    let master = MasterRng::new(params.seed);
    let biome_table =
        stages::biomes::BiomeTable::load(&params.biome_table_path, &params.tuning.biomes)?;
    let resource_config = stages::resources::ResourceConfig::load(&params.resource_config_path)?;

    let mut world = WorldData::new(bounds);

    // Stage 3.1: tectonics.
    let mut tectonic_rng = master.fork(StageId::Tectonics);
    world.tectonics = stages::tectonics::run(bounds, &params.tuning.tectonics, &mut tectonic_rng)?;

    // Stage 3.2: climate.
    let mut climate_rng = master.fork(StageId::Climate);
    world.climate = stages::climate::run(
        bounds,
        &world.tectonics,
        &params.tuning.climate,
        &mut climate_rng,
    );

    // Stage 3.3: hydrology.
    world.hydrology = stages::hydrology::run(
        bounds,
        &world.tectonics,
        &world.climate,
        &params.tuning.hydrology,
    );

    // Stage 3.4: biomes.
    let mut biome_rng = master.fork(StageId::Biomes);
    world.biomes = stages::biomes::run(
        bounds,
        &world.climate,
        &world.hydrology,
        &params.tuning.biomes,
        params.tuning.tectonics.land_level_count + 2,
        &biome_table,
        &mut biome_rng,
    );

    // Stage 3.5: resources.
    let mut resource_rng = master.fork(StageId::Resources);
    world.resources = stages::resources::run(
        stages::resources::ResourceInputs {
            tectonics: &world.tectonics,
            hydrology: &world.hydrology,
            biomes: &world.biomes,
            config: &resource_config,
        },
        &params.tuning.resources,
        params.advanced.resource_abundance,
        &mut resource_rng,
    );

    Ok(world)
}
