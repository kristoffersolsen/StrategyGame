//! City components and founding support.
//!
//! A `City` entity is the ECS representation of a founded city. It carries
//! authoritative identity via [`CityNetworkId`], ownership via
//! [`CityPlayerId`], and spatial data via [`CityCoord`] and [`OwnedTiles`].
//!
//! The centre tile's [`super::world_model::TileState::tile_owner`] is always
//! kept in sync with the city's [`CityNetworkId`]: the tile field is the
//! authoritative per-tile link; [`OwnedTiles`] is the city-side view of the
//! same relationship. See ADR-0023.

use bevy::prelude::Component;
use coords::Coord;

use crate::sim::commands::PlayerId;
use crate::sim::network_id::NetworkId;

/// Minimum hex distance that must separate any two city centres.
///
/// A founding that would place the new city at distance ≤ `MIN_CITY_SPACING`
/// from an existing city is rejected. Cities must therefore be at distance
/// strictly greater than this value to coexist.
pub(crate) const MIN_CITY_SPACING: i32 = 3;

/// Marker component for city entities.
#[derive(Component)]
pub(crate) struct City;

/// Wraps the authoritative [`NetworkId`] for a city entity.
///
/// Used by systems that need to correlate a city entity with its tile-state
/// entries (e.g. resolving `tile_owner` → owning city entity).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CityNetworkId(
    // Stage 8+: read by city-selection and tile-owner query systems.
    #[allow(dead_code)] pub(crate) NetworkId,
);

/// The [`PlayerId`] that owns this city.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CityPlayerId(
    // Stage 8+: read by ownership-display and diplomacy systems.
    #[allow(dead_code)] pub(crate) PlayerId,
);

/// Hex-grid coordinate of this city's centre tile.
///
/// Immutable after founding — cities do not move.
/// Kept in sync with the `tile_owner` entry in tile state.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CityCoord(pub(crate) Coord);

/// The set of tiles claimed by this city.
///
/// Initially contains only the centre tile. The territory system (Stage 8+)
/// will expand this over time. The authoritative per-tile ownership link is
/// [`super::world_model::TileState::tile_owner`]; this is the city-side view.
#[derive(Component, Debug, Clone)]
pub(crate) struct OwnedTiles(
    // Stage 8+: read by the territory rendering and yield systems.
    #[allow(dead_code)] pub(crate) Vec<Coord>,
);

/// Compile-time assertion that the spacing constant is positive.
const _: () = assert!(
    MIN_CITY_SPACING > 0,
    "minimum city spacing must be positive"
);
