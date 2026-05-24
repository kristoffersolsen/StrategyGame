//! Final output of the generation pipeline.

use coords::MapBounds;

/// The finalized output of the generation pipeline, consumed by the
/// `game` crate to spawn world entities.
///
/// # Current scope
///
/// As of Stage 2 this type only carries the [`MapBounds`] of the
/// generated world. Each Stage 3 sub-stage will add its own field:
///
/// - 3.1 tectonics → `elevation: Field<f32>`, `height_levels: Field<HeightLevel>`,
///   `plate_id: Field<PlateId>`, plus per-plate metadata.
/// - 3.2 climate → `temperature: Field<f32>`, `moisture: Field<f32>`,
///   `precipitation: Field<f32>`.
/// - 3.3 hydrology → `rivers`, `lakes`, `floodplains`, `canyons`.
/// - 3.4 biomes → `biome: Field<Biome>`, plus feature passes.
/// - 3.5 resources → `resources: Field<Option<Resource>>`.
/// - 3.6 starting positions → `start_positions: Vec<Coord>`.
///
/// Fields are added in pipeline order as each sub-stage lands, so the
/// shape of each sub-structure is fixed by the algorithm that fills it
/// rather than guessed up front. See `IMPLEMENTATION_PLAN.md` §3.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorldData {
    /// Map dimensions. Read by every consumer that needs to interpret
    /// any [`crate::Field`] also held in [`WorldData`].
    pub bounds: MapBounds,
}

impl WorldData {
    /// Constructs a new [`WorldData`] for a world with the given
    /// bounds and no other content.
    ///
    /// This is the constructor used by the Stage 2 stub
    /// [`crate::generate_world`] entry point; each Stage 3 sub-stage
    /// will extend it (or, more likely, a builder counterpart) with
    /// additional fields.
    #[must_use]
    pub const fn new(bounds: MapBounds) -> Self {
        Self { bounds }
    }
}
