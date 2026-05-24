//! Final output of the generation pipeline, and per-stage sub-structures.
//!
//! [`WorldData`] is the top-level type consumed by the `game` crate to
//! spawn world entities. Each pipeline stage populates its own
//! sub-structure; these live here alongside `WorldData` so that the
//! data layout is visible in one place.
//!
//! Sub-structures are added in pipeline order as each stage lands.

use coords::{Coord, MapBounds};

use crate::Field;

/// A tectonic plate identifier. Index into [`TectonicData::seeds`].
///
/// `u8` is sufficient because
/// [`MAX_TARGET_PLATE_COUNT`](crate::MAX_TARGET_PLATE_COUNT) is 32,
/// well within `u8::MAX`.
pub type PlateId = u8;

// ------------- Per-stage sub-structures -------------

/// Data produced by the tectonics stage (Stage 3.1, design §1.9).
///
/// Fields are populated incrementally as each implementation step
/// lands. Step 1 provides `seeds`; step 2/3 adds `plate_id`; later
/// steps add the elevation field and discrete height levels. See
/// `IMPLEMENTATION_PLAN.md` §3.1 for the full step roster.
#[derive(Debug, Clone)]
pub struct TectonicData {
    /// Plate seed coordinates produced by Poisson-disk sampling
    /// (step 1).
    ///
    /// Each entry is the axial coordinate of one plate's seed point.
    /// The index of a seed in this vector is its [`PlateId`].
    ///
    /// Order reflects Bridson insertion order — deterministic for a
    /// given `(params, seed)` but not geographically sorted.
    pub seeds: Vec<Coord>,

    /// Per-tile plate assignment produced by noise-perturbed Voronoi
    /// partition (steps 2–3).
    ///
    /// Each cell holds the [`PlateId`] of the plate that owns the
    /// tile. The value is an index into [`Self::seeds`]. Populated
    /// after step 3; before that, all cells are `0`.
    pub plate_id: Field<PlateId>,
}

impl TectonicData {
    /// Constructs an empty [`TectonicData`] for the given map bounds.
    ///
    /// All `plate_id` cells are initialised to `0`; `seeds` is empty.
    /// Real values are filled in by the tectonics stage steps.
    #[must_use]
    pub fn empty(bounds: MapBounds) -> Self {
        Self {
            seeds: Vec::new(),
            plate_id: Field::filled(bounds, 0),
        }
    }
}

// ------------- Top-level output -------------

/// The finalized output of the generation pipeline, consumed by the
/// `game` crate to spawn world entities.
///
/// Each Stage 3 sub-stage extends this type with a new field. The
/// sub-structures below document what each stage contributes:
///
/// - Stage 3.1 (tectonics) → [`TectonicData`]
/// - Stage 3.2 (climate) → `ClimateData` (not yet added)
/// - Stage 3.3 (hydrology) → `HydrologyData` (not yet added)
/// - Stage 3.4 (biomes) → `BiomeData` (not yet added)
/// - Stage 3.5 (resources) → `ResourceData` (not yet added)
/// - Stage 3.6 (starting positions) → `Vec<Coord>` of start positions
///
/// See `IMPLEMENTATION_PLAN.md` §3 for the full list.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorldData {
    /// Map dimensions. Required to interpret any [`crate::Field`]
    /// stored elsewhere in this struct.
    pub bounds: MapBounds,

    /// Output of the tectonics stage (design §1.9).
    pub tectonics: TectonicData,
}

impl WorldData {
    /// Constructs a [`WorldData`] with the given bounds and empty
    /// sub-structures.
    ///
    /// Called by [`crate::generate_world`] after each stage
    /// populates its sub-structure.
    #[must_use]
    pub fn new(bounds: MapBounds) -> Self {
        Self {
            bounds,
            tectonics: TectonicData::empty(bounds),
        }
    }
}
