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
