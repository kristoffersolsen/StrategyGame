//! `game`'s static terrain model and the worldgen translation step.
//!
//! [`StaticTerrain`] is `game`'s owned representation of the map's static
//! properties. It is shaped by what the game needs at runtime — not by
//! `worldgen`'s generation-internal layout — and includes precomputed
//! derivations such as per-tile movement cost.
//!
//! [`from_world_data`] is the single translation boundary: it consumes the
//! [`worldgen::WorldData`] output, copies the game-relevant fields,
//! computes derivations, and drops `WorldData`. After this call no
//! `worldgen`-internal type survives in `game`'s runtime. See ADR-0025.

use bevy::prelude::Resource;
use coords::{Coord, MapBounds};
use worldgen::{BiomeHandle, BiomeRegistry, Field, ResourceHandle, ResourceRegistry, WorldData};

// Movement cost sentinel for impassable tiles (ocean, coast, mountain, lake).
pub(crate) const IMPASSABLE: u8 = u8::MAX;

/// Classification of the transition across a shared edge between two adjacent
/// tiles, derived from their tier delta and passability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeClass {
    /// Same tier: flat meeting.
    Flat,
    /// Small passable delta: traversable ramp.
    Slope,
    /// Large delta or impassable: near-vertical cliff.
    Cliff,
}

/// `game`'s owned static terrain resource.
///
/// All fields are game-relevant. Generation intermediates (raw elevation,
/// moisture, tectonic scratch) are not present; they remain behind the
/// `worldgen` boundary. Precomputed derivations (movement cost) are stored
/// flat so hot-path queries are O(1) array reads.
///
/// Inserted by [`super::sim_startup`] and available as `Res<StaticTerrain>`
/// to all systems from world setup onward. Immutable after insertion.
#[derive(Resource)]
pub(crate) struct StaticTerrain {
    /// Map dimensions — required to interpret all `Field<T>` grids.
    pub(crate) bounds: MapBounds,

    /// Discretized elevation tier.
    ///
    /// `0` = ocean, `1` = coast, `2`–`(mountain_level-1)` = land levels,
    /// `mountain_level` = mountain. Derived from `worldgen`'s quantile-split
    /// height-level scheme; the exact value of `mountain_level` is stored
    /// separately so callers do not need to recompute it.
    pub(crate) height_level: Field<u8>,

    /// The maximum `height_level` value present on this map — the
    /// mountain tier. Precomputed at translation to avoid O(n) scans.
    pub(crate) mountain_level: u8,

    /// Hilly terrain modifier.
    ///
    /// `true` = tile has rough topography: +1 movement cost, combat bonus.
    pub(crate) is_hilly: Field<bool>,

    /// River presence.
    ///
    /// `true` = the tile carries a river channel. Derived from worldgen's
    /// `river_size` (which records small/medium/large); the exact size is not
    /// retained because current game logic only needs "has river or not." Adds
    /// a gold yield bonus; used for river-tint rendering.
    pub(crate) has_river: Field<bool>,

    /// Floodplain modifier.
    ///
    /// `true` = tile is a floodplain adjacent to a high-flow river. Adds a
    /// food yield bonus.
    pub(crate) is_floodplain: Field<bool>,

    /// Inland lake.
    ///
    /// `true` = tile is an inland lake basin. Treated as impassable like
    /// ocean.
    // Used by movement_cost computation at translation time; consulted by pathfinding (Stage 5+).
    #[allow(dead_code)]
    pub(crate) is_lake: Field<bool>,

    /// Biome classification.
    ///
    /// `None` for ocean, coast, lake, and mountain tiles — these have no
    /// biome-based yield or rendering.
    pub(crate) biome: Field<Option<BiomeHandle>>,

    /// Forest density rendering hint.
    ///
    /// Values in `[0.0, 1.0]`. Used by the 3D renderer to decide where to
    /// place tree models. Non-forest biomes and non-biome tiles receive `0.0`.
    // Reserved for the future feature render layer; terrain R0 disables that
    // layer while keeping the core terrain field.
    #[allow(dead_code)]
    pub(crate) forest_density: Field<f32>,

    /// Resource placement.
    ///
    /// `None` = no resource on this tile. At most one resource per tile.
    pub(crate) resource: Field<Option<ResourceHandle>>,

    /// Precomputed per-tile movement cost.
    ///
    /// Base movement points spent to enter this tile, combining terrain type,
    /// hilliness, and forest cover. [`IMPASSABLE`] for ocean, coast,
    /// mountain, and lake tiles. Computed once at translation; used by
    /// pathfinding (Stage 5+).
    // Stage 5+: read by the pathfinding system.
    #[allow(dead_code)]
    pub(crate) movement_cost: Field<u8>,

    /// Start positions selected by world generation.
    ///
    /// One `Coord` per player, in descending viability order. Used at game
    /// start to spawn settlers.
    pub(crate) start_positions: Vec<Coord>,

    /// Registry mapping biome string IDs to dense handles.
    ///
    /// Retained for rendering (texture lookups by biome name) and for UI
    /// display of biome labels. Crosses the boundary as-is because handles
    /// are `content::Handle` — the shared content-registry type.
    pub(crate) biome_registry: BiomeRegistry,

    /// Registry mapping resource string IDs to dense handles.
    ///
    /// Retained for rendering (feature-model placement by resource name) and
    /// UI. Same rationale as `biome_registry`.
    pub(crate) resource_registry: ResourceRegistry,
}

/// Translates `worldgen`'s output into `game`'s [`StaticTerrain`].
///
/// This is the single crossing point of the anti-corruption layer (ADR-0025).
/// `WorldData` is consumed here; no `worldgen`-internal type survives in
/// `game`'s runtime after this function returns.
///
/// Derivations computed here:
/// - `mountain_level` — max height-level value (prevents O(n) scans later).
/// - `has_river` — collapsed from `Option<RiverSize>` to `bool` (only the
///   presence of a river matters to current game logic).
/// - `movement_cost` — per-tile entry cost combining terrain, hilliness,
///   forest cover, and passability.
pub(crate) fn from_world_data(world: WorldData) -> StaticTerrain {
    let bounds = world.bounds;

    // Precompute mountain_level before we move any fields.
    let mountain_level = world
        .tectonics
        .height_level
        .iter()
        .copied()
        .max()
        .unwrap_or(0);

    // Derive `has_river`: collapse Optional<RiverSize> to bool.
    let has_river = Field::from_fn(bounds, |coord| {
        world
            .hydrology
            .river_size
            .get(coord)
            .and_then(|r| r.as_ref())
            .is_some()
    });

    // Precompute per-tile movement cost.
    let movement_cost = Field::from_fn(bounds, |coord| {
        let hl = *world.tectonics.height_level.get(coord).unwrap_or(&0);
        let hilly = *world.tectonics.is_hilly.get(coord).unwrap_or(&false);
        let lake = *world.hydrology.is_lake.get(coord).unwrap_or(&false);
        let forest = world
            .biomes
            .biome
            .get(coord)
            .copied()
            .flatten()
            .is_some_and(|h| world.biome_registry.is_forest_biome(h));
        tile_movement_cost(hl, mountain_level, hilly, forest, lake)
    });

    StaticTerrain {
        bounds,
        mountain_level,
        height_level: world.tectonics.height_level,
        is_hilly: world.tectonics.is_hilly,
        has_river,
        is_floodplain: world.hydrology.is_floodplain,
        is_lake: world.hydrology.is_lake,
        biome: world.biomes.biome,
        forest_density: world.biomes.forest_density,
        resource: world.resources.resource,
        movement_cost,
        start_positions: world.starting_positions.start_positions,
        biome_registry: world.biome_registry,
        resource_registry: world.resource_registry,
    }
}

fn tile_movement_cost(
    height_level: u8,
    mountain_level: u8,
    is_hilly: bool,
    is_forest: bool,
    is_lake: bool,
) -> u8 {
    if is_impassable_terrain(height_level, mountain_level, is_lake) {
        return IMPASSABLE;
    }
    let mut cost: u8 = 1;
    if is_hilly {
        cost = cost.saturating_add(1);
    }
    if is_forest {
        cost = cost.saturating_add(1);
    }
    cost
}

/// Classifies the edge between two adjacent tiles.
///
/// Core-owned because passability is mechanical game state: the simulation
/// decides a transition is a cliff; presentation consumes that decision.
pub(crate) fn edge_class(a: Coord, b: Coord, terrain: &StaticTerrain) -> EdgeClass {
    let a_cost = terrain.movement_cost.get(a).copied().unwrap_or(IMPASSABLE);
    let b_cost = terrain.movement_cost.get(b).copied().unwrap_or(IMPASSABLE);
    if a_cost == IMPASSABLE || b_cost == IMPASSABLE {
        return EdgeClass::Cliff;
    }

    let a_level = terrain.height_level.get(a).copied().unwrap_or(0);
    let b_level = terrain.height_level.get(b).copied().unwrap_or(0);
    match a_level.abs_diff(b_level) {
        0 => EdgeClass::Flat,
        1 => EdgeClass::Slope,
        _ => EdgeClass::Cliff,
    }
}

fn is_impassable_terrain(height_level: u8, mountain_level: u8, is_lake: bool) -> bool {
    height_level == 0 || height_level == 1 || height_level >= mountain_level || is_lake
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impassable_tiles() {
        assert_eq!(tile_movement_cost(0, 5, false, false, false), IMPASSABLE); // ocean
        assert_eq!(tile_movement_cost(1, 5, false, false, false), IMPASSABLE); // coast
        assert_eq!(tile_movement_cost(5, 5, false, false, false), IMPASSABLE); // mountain
        assert_eq!(tile_movement_cost(2, 5, false, false, true), IMPASSABLE); // lake
    }

    #[test]
    fn land_movement_costs() {
        assert_eq!(tile_movement_cost(2, 5, false, false, false), 1); // plain
        assert_eq!(tile_movement_cost(2, 5, true, false, false), 2); // hills
        assert_eq!(tile_movement_cost(2, 5, false, true, false), 2); // forest
        assert_eq!(tile_movement_cost(2, 5, true, true, false), 3); // hilly forest
    }

    fn edge_test_terrain(a_level: u8, b_level: u8, b_cost: u8) -> StaticTerrain {
        let bounds = MapBounds::new(4, 4);
        let mut height_level = Field::filled(bounds, a_level);
        height_level.set(Coord::new(1, 0), b_level);
        let mut movement_cost = Field::filled(bounds, 1);
        movement_cost.set(Coord::new(1, 0), b_cost);
        StaticTerrain {
            bounds,
            mountain_level: 5,
            height_level,
            is_hilly: Field::filled(bounds, false),
            has_river: Field::filled(bounds, false),
            is_floodplain: Field::filled(bounds, false),
            is_lake: Field::filled(bounds, false),
            biome: Field::filled(bounds, None),
            forest_density: Field::filled(bounds, 0.0),
            resource: Field::filled(bounds, None),
            movement_cost,
            start_positions: vec![],
            biome_registry: BiomeRegistry {
                inner: content::Registry::build([] as [&str; 0]),
                is_forest: vec![],
            },
            resource_registry: ResourceRegistry {
                inner: content::Registry::build([] as [&str; 0]),
                category: vec![],
            },
        }
    }

    #[test]
    fn edge_class_flat_for_same_passable_tier() {
        let terrain = edge_test_terrain(2, 2, 1);
        assert_eq!(
            edge_class(Coord::new(0, 0), Coord::new(1, 0), &terrain),
            EdgeClass::Flat
        );
    }

    #[test]
    fn edge_class_slope_for_one_step_passable_delta() {
        let terrain = edge_test_terrain(2, 3, 1);
        assert_eq!(
            edge_class(Coord::new(0, 0), Coord::new(1, 0), &terrain),
            EdgeClass::Slope
        );
    }

    #[test]
    fn edge_class_cliff_for_large_delta() {
        let terrain = edge_test_terrain(2, 4, 1);
        assert_eq!(
            edge_class(Coord::new(0, 0), Coord::new(1, 0), &terrain),
            EdgeClass::Cliff
        );
    }

    #[test]
    fn edge_class_cliff_for_impassable_tile() {
        let terrain = edge_test_terrain(2, 3, IMPASSABLE);
        assert_eq!(
            edge_class(Coord::new(0, 0), Coord::new(1, 0), &terrain),
            EdgeClass::Cliff
        );
    }
}
