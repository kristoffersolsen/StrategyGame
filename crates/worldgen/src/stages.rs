//! Pipeline-stage implementations.
//!
//! Each module corresponds to one of the six generation stages from
//! the design document. Stages run in pipeline order and write their
//! output into [`crate::WorldData`] via dedicated per-stage
//! sub-structures.
//!
//! Stages are added to this module as they are implemented; see
//! `documents/planning/implementation-plan.md` §3.

pub(crate) mod biomes;
pub(crate) mod climate;
pub(crate) mod hydrology;
pub(crate) mod resources;
pub(crate) mod tectonics;
