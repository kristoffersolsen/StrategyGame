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
//! `docs/adr/0009-determinism-and-rng-threading.md`. It will be
//! implemented in a follow-up commit; this module currently exposes
//! only the data-shape API contract that every stage will use.
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

mod error;
mod field;
mod params;
mod world_data;

pub use error::GenerationError;
pub use field::Field;
pub use params::{
    AdvancedParams, BasicParams, ClimateBand, MapArchetype, MapSize, ResourceAbundance,
    TuningParams, WorldAge, WorldParams,
};
pub use world_data::WorldData;

use coords::MapBounds;

/// Runs the full world-generation pipeline against `params` and returns
/// the finalized [`WorldData`].
///
/// This is the single entry point used by both the `game` crate (at
/// startup, on a background thread) and the `mapdump` tool (headlessly,
/// for parameter tuning).
///
/// # Errors
///
/// Returns [`GenerationError`] if any pipeline stage fails. The current
/// failure modes are:
/// - [`GenerationError::InvalidParams`] — the supplied [`WorldParams`]
///   are internally inconsistent (e.g. zero map dimensions).
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
    // Stage 2 stub: validate params, construct empty WorldData.
    // Real pipeline stages are filled in during Stage 3.1–3.6.
    params.validate()?;

    let (width, height) = params.basic.map_size.dimensions();
    // Safe: MapSize::dimensions() returns non-zero values for every
    // variant; validate() above is a defence-in-depth check that
    // catches future variants or hand-constructed params.
    let bounds = MapBounds::new(width, height);

    Ok(WorldData::new(bounds))
}
