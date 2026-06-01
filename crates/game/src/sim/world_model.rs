//! Dynamic per-tile runtime state resource.
//!
//! [`TileState`] is the mutable half of the runtime world model. The static
//! half is [`super::terrain::StaticTerrain`].
//!
//! Tiles are not ECS entities. Only units and cities are entities.
//! See ADR-0023 for the full rationale.

use bevy::prelude::Resource;
use content::Handle as ContentHandle;
use coords::MapBounds;
use worldgen::Field;

use crate::sim::network_id::NetworkId;

/// Mutable per-tile runtime state for the whole map.
///
/// A set of parallel [`Field<T>`] grids, each sized to the full map at
/// construction. Reads and writes are by [`coords::Coord`]. Authoritative
/// cross-tile references are stored as [`NetworkId`]s; Bevy [`Entity`]
/// indices are not stored here. See ADR-0023 and ADR-0024.
#[derive(Resource)]
pub(crate) struct TileState {
    /// The [`NetworkId`] of the city that owns this tile (`None` = unclaimed).
    ///
    /// Written when a city is founded or territory changes hands (Stage 7+).
    // Stage 7+: written by the FoundCity command processor.
    #[allow(dead_code)]
    pub(crate) tile_owner: Field<Option<NetworkId>>,

    /// The [`NetworkId`] of the unit occupying this tile (`None` = unoccupied).
    ///
    /// Written by `MoveUnit` and `FoundCity` command processors (Stage 6+).
    // Stage 6+: written by MoveUnit and FoundCity command processors.
    #[allow(dead_code)]
    pub(crate) tile_occupant: Field<Option<NetworkId>>,

    /// Content handle of the player-built improvement on this tile.
    ///
    /// At most one improvement per tile is a deliberate design constraint
    /// (ADR-0023). `None` = no improvement. Populated by Stage 8+ worker
    /// commands.
    // Stage 8+: read by tile_yield to apply the improvement transform.
    #[allow(dead_code)]
    pub(crate) improvement: Field<Option<ContentHandle>>,

    /// Infrastructure bit-flags for this tile (roads, irrigation, etc.).
    ///
    /// Specific bit assignments are reserved for Stage 8+ when the
    /// infrastructure system is designed.
    // Stage 8+: bit assignments defined when the infrastructure system is designed.
    #[allow(dead_code)]
    pub(crate) infrastructure: Field<u8>,

    /// Whether the tile's native forest cover has been cleared by a worker.
    ///
    /// Clearing suppresses the forest-density bonus when computing tile yield.
    /// Populated by Stage 8+ worker commands.
    // Stage 8+: read by tile_yield to suppress the forest-density yield component.
    #[allow(dead_code)]
    pub(crate) cleared: Field<bool>,
}

impl TileState {
    /// Constructs a [`TileState`] with all grids empty or zero-initialised
    /// for the given map bounds.
    pub(crate) fn new(bounds: MapBounds) -> Self {
        Self {
            tile_owner: Field::filled(bounds, None),
            tile_occupant: Field::filled(bounds, None),
            improvement: Field::filled(bounds, None),
            infrastructure: Field::filled(bounds, 0),
            cleared: Field::filled(bounds, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::{Coord, MapBounds};

    #[test]
    fn tile_state_initialises_all_fields_to_empty() {
        let bounds = MapBounds::new(10, 5);
        let state = TileState::new(bounds);
        let c = Coord::new(3, 2);

        assert_eq!(state.tile_owner.get(c), Some(&None));
        assert_eq!(state.tile_occupant.get(c), Some(&None));
        assert_eq!(state.improvement.get(c), Some(&None));
        assert_eq!(state.infrastructure.get(c), Some(&0_u8));
        assert_eq!(state.cleared.get(c), Some(&false));
    }

    #[test]
    fn tile_state_covers_full_bounds() {
        let bounds = MapBounds::new(4, 4);
        let state = TileState::new(bounds);
        // All corners should be accessible.
        for c in [
            Coord::new(0, 0),
            Coord::new(3, 0),
            Coord::new(0, 3),
            Coord::new(3, 3),
        ] {
            assert!(state.tile_owner.get(c).is_some());
            assert!(state.tile_occupant.get(c).is_some());
            assert!(state.improvement.get(c).is_some());
            assert!(state.infrastructure.get(c).is_some());
            assert!(state.cleared.get(c).is_some());
        }
    }
}
