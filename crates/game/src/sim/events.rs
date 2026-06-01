//! Game result messages emitted by the command processor.
//!
//! All types implement [`bevy::prelude::Message`] for buffered,
//! schedule-placed delivery — not observer-triggered. Consumers
//! (rendering, UI, audio) read these messages; only the command processor
//! writes them. This is the "buffered events, not observers" decision
//! recorded in `documents/planning/implementation_plan2.md`.

use coords::Coord;

use crate::sim::commands::PlayerId;
use crate::sim::network_id::NetworkId;

/// Emitted when a command is rejected by the command processor.
///
/// State is unchanged when this is emitted. The `reason` field
/// describes the specific validation failure.
// Stage 6+: read by the input tier to surface rejection feedback to the player.
#[allow(dead_code)]
#[derive(bevy::prelude::Message, Debug, Clone)]
pub(crate) struct CommandRejected {
    /// Why the command was rejected.
    pub(crate) reason: RejectionReason,
}

/// Why a command was rejected by the processor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RejectionReason {
    /// No occupant with the given [`NetworkId`] was found on any tile.
    UnitNotFound,
    /// The destination tile is impassable (ocean, coast, mountain, or lake).
    DestinationImpassable,
    /// The destination tile is already occupied by another unit (1UPT violation).
    DestinationOccupied,
    /// The unit is not owned by the player that issued the command.
    NotYourUnit,
    /// The unit does not have enough movement points remaining this turn.
    InsufficientMovementPoints,
}

/// Emitted when a unit moves successfully.
///
/// Defined here as the reaction contract (Stage 5).
/// Consumed by unit rendering (Stage 6+).
// Stage 6+: fields read by the unit rendering system to update entity positions.
#[allow(dead_code)]
#[derive(bevy::prelude::Message, Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnitMoved {
    /// The unit that moved.
    pub(crate) unit_id: NetworkId,
    /// The player that issued the move.
    pub(crate) player_id: PlayerId,
    /// The tile the unit moved from.
    pub(crate) from: Coord,
    /// The tile the unit moved to.
    pub(crate) to: Coord,
}

/// Emitted when a city is founded successfully.
///
/// Defined here as the reaction contract (Stage 5).
/// Consumed by city rendering (Stage 7+).
// Stage 7+: emitted and consumed by the FoundCity command path.
#[allow(dead_code)]
#[derive(bevy::prelude::Message, Debug, Clone)]
pub(crate) struct CityFounded {
    /// The newly created city.
    pub(crate) city_id: NetworkId,
    /// The player that founded the city.
    pub(crate) player_id: PlayerId,
    /// The tile where the city was founded.
    pub(crate) coord: Coord,
}
