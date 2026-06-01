//! Player identity, command enum, and command queue.
//!
//! [`GameCommand`] is the sum type of all mutations a command source may
//! request. [`CommandQueue`] is the FIFO buffer between sources and the
//! processor. [`PlayerId`] is the stable player identifier carried by every
//! command so the processor can enforce ownership rules.

use std::collections::VecDeque;

use bevy::prelude::Resource;
use coords::Coord;

use crate::sim::network_id::NetworkId;

/// Stable identifier for a player in this game session.
///
/// Assigned at game setup; constant for the session lifetime.
/// Player `0` is the local player in a single-player game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PlayerId(pub(crate) u32);

/// All commands a command source may submit.
///
/// Every variant carries a [`PlayerId`] so the processor can enforce
/// ownership rules. Commands are pushed by sources (player input, AI) and
/// popped by the [`super::processor`]. No system outside the processor ever
/// mutates game state directly.
#[derive(Debug, Clone)]
pub(crate) enum GameCommand {
    /// Move a unit to `destination`.
    ///
    /// Validated by the processor:
    /// - Source tile must have an occupant matching `unit_id`.
    /// - Destination must be passable and unoccupied (1UPT).
    MoveUnit {
        /// Player issuing the command; must own the unit.
        player_id: PlayerId,
        /// Identifies the unit via the occupant grid.
        unit_id: NetworkId,
        /// Target tile. Must be passable; 1UPT is enforced.
        destination: Coord,
    },

    /// Found a city at the settler's current position.
    ///
    /// Validated by the processor:
    /// - `settler_id` must identify a settler owned by `player_id`.
    /// - The settler's tile must be passable (not ocean/coast/mountain/lake).
    /// - No existing city may be within [`super::cities::MIN_CITY_SPACING`]
    ///   tiles.
    ///
    /// On success: the settler is consumed, a `City` entity is spawned, and
    /// the tile's `tile_owner` is set to the new city's [`NetworkId`].
    FoundCity {
        /// Player issuing the command; must own the settler.
        player_id: PlayerId,
        /// The settler unit to consume.
        settler_id: NetworkId,
    },
}

/// FIFO queue of pending [`GameCommand`]s.
///
/// Command sources push onto the back; the processor pops from the front.
/// FIFO order guarantees deterministic processing — every machine running the
/// same sequence produces identical state.
#[derive(Resource, Default)]
pub(crate) struct CommandQueue {
    pending: VecDeque<GameCommand>,
}

impl CommandQueue {
    /// Enqueues `command` at the back of the queue.
    pub(crate) fn push(&mut self, command: GameCommand) {
        self.pending.push_back(command);
    }

    /// Removes and returns the front command, or `None` if the queue is empty.
    pub(crate) fn pop(&mut self) -> Option<GameCommand> {
        self.pending.pop_front()
    }

    /// Returns `true` if there are no pending commands.
    // Stage 6+: queried by the turn-advance gate.
    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Returns the number of pending commands.
    // Stage 6+: diagnostic / UI display.
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::Coord;

    use crate::sim::network_id::NetworkIdAllocator;

    #[test]
    fn command_queue_is_fifo() {
        let mut queue = CommandQueue::default();
        let mut alloc = NetworkIdAllocator::default();
        let unit_a = alloc.next();
        let unit_b = alloc.next();

        queue.push(GameCommand::MoveUnit {
            player_id: PlayerId(0),
            unit_id: unit_a,
            destination: Coord::new(1, 0),
        });
        queue.push(GameCommand::MoveUnit {
            player_id: PlayerId(0),
            unit_id: unit_b,
            destination: Coord::new(2, 0),
        });

        assert_eq!(queue.len(), 2);

        let first = queue.pop().unwrap();
        let GameCommand::MoveUnit { unit_id, .. } = first else {
            panic!("expected MoveUnit");
        };
        assert_eq!(unit_id, unit_a, "first out must be first in");

        let second = queue.pop().unwrap();
        let GameCommand::MoveUnit { unit_id, .. } = second else {
            panic!("expected MoveUnit");
        };
        assert_eq!(unit_id, unit_b, "second out must be second in");

        assert!(queue.is_empty());
    }

    #[test]
    fn pop_on_empty_queue_returns_none() {
        let mut queue = CommandQueue::default();
        assert!(queue.pop().is_none());
    }
}
