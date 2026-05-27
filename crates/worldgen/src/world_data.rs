//! Final output of the generation pipeline, and per-stage sub-structures.
//!
//! [`WorldData`] is the top-level type consumed by the `game` crate to
//! spawn world entities. Each pipeline stage populates its own
//! sub-structure; these live here alongside `WorldData` so that the
//! data layout is visible in one place.
//!
//! Sub-structures are added in pipeline order as each stage lands.

use coords::{Coord, Direction, MapBounds};

use crate::Field;

// ------------- Biome classification -------------

/// Land biome assigned from temperature and moisture.
///
/// Water, coast, lake, and mountain tiles receive no biome assignment;
/// those are terrain or hydrology states owned by earlier stages.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Biome {
    /// Hot and wet forest biome.
    TropicalRainforest,
    /// Moderate-temperature forest biome.
    TemperateForest,
    /// Cold taiga forest biome.
    BorealForest,
    /// Open grassland or savanna biome.
    Grassland,
    /// Low-moisture biome at any temperature.
    Desert,
    /// Cold sparse-ground biome.
    Tundra,
}

impl Biome {
    /// Returns `true` if this biome supports forest-density output.
    #[must_use]
    pub const fn is_forest(self) -> bool {
        matches!(
            self,
            Self::TropicalRainforest | Self::TemperateForest | Self::BorealForest
        )
    }
}

// ------------- Boundary classification -------------

/// The interaction type at a boundary between two adjacent tectonic plates.
///
/// Determined by the component of the relative motion vector directed
/// along the boundary normal (the unit vector from one plate's seed to
/// the other's). Given plates A and B with seeds and motion vectors,
/// the approach velocity is `dot(motion_A − motion_B, normal_A→B)`:
///
/// - Positive and above `convergence_threshold` → [`Self::Convergent`].
/// - Negative and below the negative threshold → [`Self::Divergent`].
/// - Otherwise → [`Self::Transform`].
///
/// Stored per tile in [`TectonicData::boundary_type`] and used in step 6
/// to select the elevation modifier for each boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryType {
    /// Plates moving toward each other.
    ///
    /// Continental–continental: produces mountain ranges.
    /// Continental–oceanic: produces coastal mountains and subduction trenches.
    /// Oceanic–oceanic: produces island arcs.
    Convergent,
    /// Plates moving apart.
    ///
    /// Continental–continental: produces rift valleys.
    /// Oceanic–oceanic: produces mid-ocean ridges.
    Divergent,
    /// Relative motion is mostly tangential — plates sliding past each
    /// other. Produces minor faulting but no significant elevation change.
    Transform,
}

/// A tectonic plate identifier. Index into [`TectonicData::seeds`]
/// and [`TectonicData::plates`].
///
/// `u8` is sufficient because
/// [`crate::params::MAX_TARGET_PLATE_COUNT`] is 32,
/// well within `u8::MAX`.
pub type PlateId = u8;

// ------------- Plate properties -------------

/// The geological type of a tectonic plate.
///
/// Determines the plate's base elevation before boundary effects and
/// noise are applied. Continental plates sit above the notional zero
/// line; oceanic plates sit below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateType {
    /// Lighter silicic crust. Base elevation is positive, producing
    /// land when sea level is applied.
    Continental,
    /// Denser mafic crust. Base elevation is negative, producing
    /// ocean when sea level is applied.
    Oceanic,
}

/// The dominant mineral character of a tectonic plate.
///
/// Used by the resource placement stage (design §1.13) to give plates
/// characteristic resource monopolies — iron-rich plates produce more
/// strategic metals, gem-bearing plates produce luxury deposits, and so
/// on. The exact resource mapping is content data (RON config); this
/// enum is the generation-time token that drives the lookup.
///
/// Variants are intentionally coarse geological categories rather than
/// specific resources, so the mapping from character to resource can be
/// tuned independently of generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MineralCharacter {
    /// Rich in ferrous metals (iron, nickel). Common.
    Ferrous,
    /// Rich in precious metals (gold, silver, copper). Uncommon.
    Precious,
    /// Rich in gemstones (diamonds, emeralds). Rare.
    Gems,
    /// Rich in hydrocarbon deposits (oil, coal, gas). Common.
    Hydrocarbons,
    /// Rich in industrial minerals (stone, salt, clay). Common.
    Industrial,
}

impl MineralCharacter {
    /// All variants in definition order, used for weighted random
    /// selection.
    pub(crate) const ALL: &'static [Self] = &[
        Self::Ferrous,
        Self::Precious,
        Self::Gems,
        Self::Hydrocarbons,
        Self::Industrial,
    ];
}

/// A 2D motion vector representing the notional drift of a tectonic
/// plate.
///
/// The components are dimensionless — the scale has no physical
/// meaning. Only the *relative* motion between adjacent plates matters
/// for boundary classification (step 5): whether two plates are moving
/// toward, away from, or parallel to each other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionVector {
    /// East-west component. Positive is eastward.
    pub dx: f32,
    /// North-south component. Positive is southward.
    pub dy: f32,
}

/// The full set of properties assigned to a single tectonic plate in
/// step 4.
///
/// Indexed by [`PlateId`] in [`TectonicData::plates`]. The
/// corresponding seed coordinate is at the same index in
/// [`TectonicData::seeds`].
#[derive(Debug, Clone)]
pub struct Plate {
    /// Geological type, determining base elevation.
    pub plate_type: PlateType,
    /// Notional drift vector for boundary classification.
    pub motion: MotionVector,
    /// Dominant mineral character for resource placement.
    pub mineral_character: MineralCharacter,
}

// ------------- Per-stage sub-structures -------------

/// Data produced by the tectonics stage (Stage 3.1, design §1.9).
///
/// Fields are populated incrementally by each implementation step:
/// - Step 1: [`Self::seeds`]
/// - Steps 2–3: [`Self::plate_id`]
/// - Step 4: [`Self::plates`]
/// - Step 5: [`Self::boundary_type`]
/// - Steps 6–7: [`Self::elevation`]
/// - Step 8: [`Self::height_level`]
/// - Step 9: [`Self::is_hilly`]
///
/// See `documents/planning/implementation-plan.md` §3.1 for the full step roster.
#[derive(Debug, Clone)]
pub struct TectonicData {
    /// Plate seed coordinates produced by Poisson-disk sampling (step 1).
    ///
    /// Each entry is the axial coordinate of one plate's seed point.
    /// The index of a seed in this vector is its [`PlateId`].
    ///
    /// Order reflects Bridson insertion order — deterministic for a
    /// given `(params, seed)` but not geographically sorted.
    pub seeds: Vec<Coord>,

    /// Per-tile plate assignment produced by organic flood-fill growth
    /// from plate seeds (step 2).
    ///
    /// Each cell holds the [`PlateId`] of the plate that owns the tile.
    /// The value is an index into [`Self::seeds`] and [`Self::plates`].
    pub plate_id: Field<PlateId>,

    /// Per-plate properties assigned in step 4.
    ///
    /// Indexed by [`PlateId`]. Parallel to [`Self::seeds`]: entry `i`
    /// describes the plate whose seed is at `seeds[i]`.
    pub plates: Vec<Plate>,

    /// Per-tile boundary classification produced in step 5.
    ///
    /// `None` for interior tiles (no adjacent tile belongs to a
    /// different plate). `Some(`[`BoundaryType`]`)` for boundary tiles,
    /// carrying the most geologically significant boundary type among
    /// all adjacent plate pairs (convergent > divergent > transform).
    pub boundary_type: Field<Option<BoundaryType>>,

    /// Continuous elevation field produced by steps 6–7 and normalised
    /// in step 8 so that sea level = 0.
    ///
    /// Values are dimensionless with typical range roughly ±1.5.
    /// Positive values are above sea level; negative values are below.
    pub elevation: Field<f32>,

    /// Discrete height-level assigned to each tile in step 8.
    ///
    /// Encoding (for the default `land_level_count = 3`):
    /// `0` = ocean, `1` = coast, `2`–`4` = land levels (low to high),
    /// `5` = mountain. The level count is `land_level_count + 3`;
    /// thresholds are computed by quantile-matching `level_proportions`.
    pub height_level: Field<u8>,

    /// Per-tile roughness attribute produced in step 9.
    ///
    /// `true` means the tile is "hilly": it reduces movement speed and
    /// provides a combat bonus, independent of its height level. Hills
    /// are more common near tectonic boundaries and are spatially
    /// correlated via a noise field.
    pub is_hilly: Field<bool>,
}

impl TectonicData {
    /// Constructs an empty [`TectonicData`] for the given map bounds.
    ///
    /// All fields are empty or zero-initialised. Real values are filled
    /// in by the tectonics stage steps.
    #[must_use]
    pub fn empty(bounds: MapBounds) -> Self {
        Self {
            seeds: Vec::new(),
            plate_id: Field::filled(bounds, 0),
            plates: Vec::new(),
            boundary_type: Field::filled(bounds, None),
            elevation: Field::filled(bounds, 0.0),
            height_level: Field::filled(bounds, 0),
            is_hilly: Field::filled(bounds, false),
        }
    }
}

// ------------- Climate sub-structure -------------

/// Data produced by the climate stage (Stage 3.2, design §1.10).
///
/// All four fields are populated in a single pass by the climate entry
/// point after the advection loop converges. The precipitation field
/// is accumulated across all advection iterations.
#[derive(Debug, Clone)]
pub struct ClimateData {
    /// Per-tile normalised temperature.
    ///
    /// Derived from latitude and elevation in step 1. Values are
    /// dimensionless with range roughly `[pole_temperature,
    /// equator_temperature]` (typically `[−0.5, 1.0]` at defaults).
    pub temperature: Field<f32>,

    /// Per-tile steady-state moisture content.
    ///
    /// Populated by the advection loop (steps 4–6). Values are
    /// dimensionless; the scale reflects accumulated evaporation
    /// minus precipitation over all iterations.
    pub moisture: Field<f32>,

    /// Per-tile 2D wind velocity.
    ///
    /// Stored as `[east_component, south_component]` in the hex
    /// Cartesian frame (see ADR-0005 for the frame definition).
    /// Magnitudes are bounded by the CFL cap.
    pub wind: Field<[f32; 2]>,

    /// Per-tile accumulated precipitation over all iterations.
    ///
    /// Summed across every advection iteration. Not normalised per
    /// iteration, so absolute values depend on `iteration_count`.
    /// Used for biome assignments and rain-shadow visualisation.
    pub precipitation: Field<f32>,
}

impl ClimateData {
    /// Constructs an empty [`ClimateData`] for the given map bounds.
    ///
    /// All fields are zero-initialised. Real values are filled in by
    /// the climate stage.
    #[must_use]
    pub fn empty(bounds: MapBounds) -> Self {
        Self {
            temperature: Field::filled(bounds, 0.0),
            moisture: Field::filled(bounds, 0.0),
            wind: Field::filled(bounds, [0.0, 0.0]),
            precipitation: Field::filled(bounds, 0.0),
        }
    }
}

// ------------- Hydrology sub-structure -------------

/// Size category assigned to tiles whose accumulated flow forms a river.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiverSize {
    /// Small river source or tributary.
    Creek,
    /// Established river channel.
    River,
    /// High-flow trunk river.
    MajorRiver,
}

/// Data produced by the hydrology stage (Stage 3.3, design §1.11).
///
/// The stage consumes tectonic elevation and climate precipitation, then
/// produces drainage directions, lakes, rivers, floodplains, canyons,
/// canyon walls, and a temporary post-hydrology height-level copy.
#[derive(Debug, Clone)]
pub struct HydrologyData {
    /// Per-tile downstream flow direction.
    ///
    /// `None` means the tile is a terminal: ocean, coast, or lake.
    pub flow_direction: Field<Option<Direction>>,

    /// Normalised precipitation-weighted flow accumulation.
    ///
    /// Values are scaled to `[0.0, 1.0]` by the stage-wide maximum so
    /// river thresholds remain map-size independent.
    pub flow_accumulation: Field<f32>,

    /// Tiles marked as inland lakes by basin resolution.
    pub is_lake: Field<bool>,

    /// Spill-point outlet coordinate for each lake tile.
    ///
    /// `Some(coord)` is the first land tile outside the lake basin where
    /// accumulated flow exits.  `None` means the lake is a true sink (no
    /// outside neighbor — fully enclosed map edge case).  Non-lake tiles
    /// always hold `None`.
    pub lake_outlet: Field<Option<Coord>>,

    /// River classification for each tile, if accumulated flow crosses
    /// the configured river threshold.
    pub river_size: Field<Option<RiverSize>>,

    /// Tiles adjacent to river channels and close enough in elevation
    /// to be treated as floodplains.
    pub is_floodplain: Field<bool>,

    /// River tiles lowered by the canyon carving pass.
    pub is_canyon: Field<bool>,

    /// Tiles flanking carved canyons.
    pub is_canyon_wall: Field<bool>,

    /// Temporary post-hydrology height-level field.
    ///
    /// See ADR-0012. This duplicates [`TectonicData::height_level`]
    /// during hydrology and biome tuning so debug output can compare
    /// pre- and post-hydrology elevation classes.
    pub height_level: Field<u8>,
}

impl HydrologyData {
    /// Constructs an empty [`HydrologyData`] for the given map bounds.
    #[must_use]
    pub fn empty(bounds: MapBounds) -> Self {
        Self {
            flow_direction: Field::filled(bounds, None),
            flow_accumulation: Field::filled(bounds, 0.0),
            is_lake: Field::filled(bounds, false),
            lake_outlet: Field::filled(bounds, None),
            river_size: Field::filled(bounds, None),
            is_floodplain: Field::filled(bounds, false),
            is_canyon: Field::filled(bounds, false),
            is_canyon_wall: Field::filled(bounds, false),
            height_level: Field::filled(bounds, 0),
        }
    }
}

// ------------- Biome sub-structure -------------

/// Data produced by the biome stage (Stage 3.4, design §1.12).
#[derive(Debug, Clone)]
pub struct BiomeData {
    /// Per-land-tile biome classification.
    ///
    /// `None` for ocean, coast, lakes, and mountain-level tiles.
    pub biome: Field<Option<Biome>>,

    /// Rendering hint for forest coverage density.
    ///
    /// Values are in `[0.0, 1.0]`. Non-forest biomes and non-biome
    /// tiles receive `0.0`.
    pub forest_density: Field<f32>,
}

impl BiomeData {
    /// Constructs an empty [`BiomeData`] for the given map bounds.
    #[must_use]
    pub fn empty(bounds: MapBounds) -> Self {
        Self {
            biome: Field::filled(bounds, None),
            forest_density: Field::filled(bounds, 0.0),
        }
    }
}

// ------------- Top-level output -------------

/// The finalized output of the generation pipeline, consumed by the
/// `game` crate to spawn world entities.
///
/// Each Stage 3 sub-stage extends this type with a new field:
///
/// - Stage 3.1 (tectonics) → [`TectonicData`]
/// - Stage 3.2 (climate) → [`ClimateData`]
/// - Stage 3.3 (hydrology) → [`HydrologyData`]
/// - Stage 3.4 (biomes) → [`BiomeData`]
/// - Stage 3.5 (resources) → `ResourceData` (not yet added)
/// - Stage 3.6 (starting positions) → `Vec<Coord>` (not yet added)
///
/// See `documents/planning/implementation-plan.md` §3 for the full list.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorldData {
    /// Map dimensions. Required to interpret any [`crate::Field`]
    /// stored elsewhere in this struct.
    pub bounds: MapBounds,

    /// Output of the tectonics stage (design §1.9).
    pub tectonics: TectonicData,

    /// Output of the climate stage (design §1.10).
    pub climate: ClimateData,

    /// Output of the hydrology stage (design §1.11).
    pub hydrology: HydrologyData,

    /// Output of the biome stage (design §1.12).
    pub biomes: BiomeData,
}

impl WorldData {
    /// Constructs a [`WorldData`] with the given bounds and empty
    /// sub-structures.
    #[must_use]
    pub fn new(bounds: MapBounds) -> Self {
        Self {
            bounds,
            tectonics: TectonicData::empty(bounds),
            climate: ClimateData::empty(bounds),
            hydrology: HydrologyData::empty(bounds),
            biomes: BiomeData::empty(bounds),
        }
    }
}
