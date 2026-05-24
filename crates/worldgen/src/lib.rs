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
//! See `docs/design/overview.md` §1.9–1.14 for the pipeline design.

// TODO(kristoffer): implement WorldParams fully in Stage 2 (§2.1)
/// Input parameters controlling world generation.
///
/// This is a placeholder. The full type — with basic, advanced, and
/// full-tunability layers — is specified in Stage 2 of the implementation
/// plan (§2.1) and the design doc (§1.15).
#[derive(Debug, Default)]
pub struct WorldParams {
    /// Master RNG seed.
    pub seed: u64,
}

// TODO(kristoffer): implement WorldData fully in Stage 2 (§2.1)
/// The finalized output of the generation pipeline.
///
/// This is a placeholder. The full type — holding heightmap, climate
/// fields, biome map, resources, and starting positions — is specified
/// in Stage 2 of the implementation plan (§2.1) and the design doc (§1.8).
#[derive(Debug)]
pub struct WorldData;
