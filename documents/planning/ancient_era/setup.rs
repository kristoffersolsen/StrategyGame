//! Game setup: building the initial authoritative state (`state_0`).
//!
//! Intended path `game::sim::setup`. Runs once at game start, before the first
//! [`Interactive`](crate::sim::turn::TurnPhase::Interactive) phase. It consumes the
//! transient [`WorldData`] (ADR-0025 — a setup input, drained and discarded, never
//! a long-lived resource) into the authoritative dense tile state (ADR-0023), then
//! spawns the entities the turn loop operates on: empires, their starting
//! settlements and units, and the initial trade-network partition.
//!
//! The roster is **fixed here and never grows** (no mid-game joins), which is what
//! lets [`EmpireId`] be a dense `u8` for the whole game. Each roster entry pairs an
//! id with a [`ControllerMode`] (human or AI) and — once the civilization-identity
//! ADR lands (ADR-0022 open-kinds) — a civilization the empire embodies.
//!
//! Scope honesty: this scaffolds the *shape* of setup. Its *content* is gated on
//! work not yet done — **where** starting settlements go is Stage 3.6 (starting
//! positions, unbuilt), and **what** starting units/settlers each empire gets is
//! pending tech-tree and create/upkeep cost tuning. Those are `todo!()` with the
//! dependency named, not guessed.

use bevy::prelude::Component;
use worldgen::WorldData;

use crate::sim::empire::{ControllerMode, EmpireId};

/// The fixed roster and world the game is instantiated from. Drained by setup.
#[derive(Component)]
pub(crate) struct GameSetup {
    /// The generated world — consumed into tile state, then dropped (ADR-0025).
    pub(crate) world: WorldData,
    /// One entry per empire; fixes the roster for the whole game.
    pub(crate) roster: Vec<EmpireSetup>,
}

/// One empire's setup: its id, who drives it, and (later) the civilization it
/// embodies.
pub(crate) struct EmpireSetup {
    /// Dense id, stable for the game.
    pub(crate) id: EmpireId,
    /// Human or AI at start (switchable later by command).
    pub(crate) controller: ControllerMode,
    // TODO(kristoffer): civilization identity (ADR-0022 open-kinds) once that ADR
    // lands — the composable identity the empire embodies.
}

// ------------- Setup systems (run once, in order) -------------

/// Drains [`WorldData`] into the authoritative dense tile state, then discards it.
///
/// Tile terrain, features, resources, rivers, and road `infrastructure` become the
/// dense grid the simulation owns (ADR-0023); `WorldData` is not retained
/// afterward (ADR-0025). The renderer's derived tile index is built from this state
/// downstream, not from `WorldData`.
pub(crate) fn consume_world() {
    todo!("drain WorldData into dense tile state (ADR-0023), then drop it (ADR-0025)")
}

/// Spawns one entity per roster entry: `Empire`, [`EmpireId`], [`ControllerMode`],
/// `Treasury::default()`, and an empty `transit::RoadPool`.
///
/// These are the owner entities everything else is tagged against
/// (`empire::Owner`). Deterministic: spawned in ascending [`EmpireId`].
pub(crate) fn spawn_empires() {
    todo!("one entity per roster entry: Empire, EmpireId, ControllerMode, Treasury, RoadPool")
}

/// Spawns each empire's starting settlement(s) at their starting positions.
///
/// Each carries `Settlement`, `empire::Owner`, `SettlementCenter`,
/// `SettlementEconomy`, `Storage`, `SettlementNetworkId`, and a
/// `TradeNetworkId` (stamped by [`seed_trade_networks`]). Claims the center's
/// initial tiles into the dense ownership grid.
///
/// **Where** is Stage 3.6 (starting positions, not yet built); this depends on it.
pub(crate) fn spawn_starting_settlements() {
    todo!("place per Stage 3.6 starting positions; spawn settlement entity bundle")
}

/// Spawns each empire's starting units (e.g. a worker, a warrior, a scout),
/// owned via `empire::Owner`, at/near its starting settlement.
///
/// **What** the opening loadout is depends on tech-tree boundaries and worker
/// create/upkeep cost tuning (both deferred); the count and kinds are not guessed
/// here.
pub(crate) fn spawn_starting_units() {
    todo!("opening unit loadout pending tech-tree + create/upkeep cost tuning")
}

/// Stamps the initial trade-network partition: with no roads yet, every starting
/// settlement is its own singleton component.
///
/// Calls the same [`recompute_trade_networks`](crate::sim::transit::recompute_trade_networks)
/// each empire will use thereafter — setup is not a special partition path.
pub(crate) fn seed_trade_networks() {
    todo!("recompute_trade_networks per empire; singletons until roads exist")
}
