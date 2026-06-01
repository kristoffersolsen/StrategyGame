//! Authoritative simulation core — the trust-tier heart of the game.
//!
//! The `sim` module is the only tier permitted to mutate authoritative game
//! state. Command sources (`input`, future `ai`) observe state and *propose*
//! changes by issuing commands; the presentation tier (`rendering`, UI)
//! consumes the messages the core emits. Neither tier ever mutates state
//! directly.
//!
//! See the "Runtime architecture: trust tiers and the headless core" section
//! of `documents/planning/implementation_plan2.md` for the full design.
//!
//! # Stage 4 / 4.5 contents
//!
//! - [`terrain::StaticTerrain`] — game's owned static terrain model, built
//!   from worldgen output at startup and immutable thereafter.
//! - [`world_model::TileState`] — mutable per-tile state (parallel
//!   `Field<T>` grids, one per dynamic attribute).
//! - [`network_id::NetworkId`] — deterministic cross-machine actor id.
//! - [`network_id::NetworkIdAllocator`] — monotonic counter for id allocation.
//! - [`network_id::EntityLookup`] — derived non-authoritative id→entity cache.
//! - [`yield_table::YieldTable`] — dense per-biome and per-feature yield data.
//! - [`yield_table::tile_yield`] — the shared tile-yield function.
//!
//! # Stage 5 contents
//!
//! - [`turn_phase::TurnPhase`] — sub-state driving the player-input /
//!   resolution cycle.
//! - [`turn_phase::TurnState`] — current turn counter.
//! - [`turn_phase::EndTurnRequested`] — message written by command sources
//!   to advance the turn.
//! - [`commands::PlayerId`] — stable player identifier.
//! - [`commands::GameCommand`] — the command sum type.
//! - [`commands::CommandQueue`] — FIFO buffer between sources and processor.
//! - [`events::CommandRejected`] — emitted by the processor for invalid commands.
//! - [`events::UnitMoved`] — emitted after a successful `MoveUnit`.
//! - [`events::CityFounded`] — reaction contract for Stage 7.
//! - [`processor::process_commands`] — drains and applies the queue.
//!
//! # Stage 6 contents
//!
//! - [`units::Unit`] — marker component for unit entities.
//! - [`units::UnitKind`] — unit type enum.
//! - [`units::UnitNetworkId`] — wraps the authoritative network id.
//! - [`units::UnitPlayerId`] — ownership component.
//! - [`units::UnitCoord`] — hex-grid position component.
//! - [`units::MovementPoints`] — per-turn movement budget component.
//! - [`units::spawn_settlers`] — spawns the initial settler at startup.
//! - [`units::reset_movement_points`] — resets movement budget at turn start.
//! - Startup scheduling initialises static terrain, runtime tile state, and
//!   initial units before presentation-owned state transitions run.

mod commands;
mod events;
mod network_id;
pub(crate) mod processor;
pub(crate) mod terrain;
mod turn_phase;
pub(crate) mod units;
mod world_model;
mod yield_table;

pub(crate) use commands::{CommandQueue, GameCommand, PlayerId};
pub(crate) use events::{CommandRejected, UnitMoved};
pub(crate) use network_id::EntityLookup;
pub(crate) use terrain::StaticTerrain;
pub(crate) use turn_phase::{EndTurnRequested, TurnPhase, TurnState};
pub(crate) use units::{UnitCoord, UnitNetworkId};
pub(crate) use world_model::TileState;

use std::path::Path;

use bevy::prelude::{
    App, AppExtStates, Commands, IntoScheduleConfigs, OnEnter, Plugin, Res, Resource, Startup,
    Update, in_state,
};
use tracing::info;
use worldgen::{WorldParams, generate_world};

use events::CityFounded;
use network_id::NetworkIdAllocator;
use yield_table::tile_yield;

/// Default world-generation parameter asset, relative to the workspace root.
const DEFAULT_WORLD_PARAMS_PATH: &str = "assets/worldgen/default_params.ron";

/// Startup inputs for the authoritative simulation core.
#[derive(Resource, Debug, Clone)]
pub struct SimStartupConfig {
    /// World-generation parameters used to create the initial map.
    pub world_params: WorldParams,
}

impl Default for SimStartupConfig {
    fn default() -> Self {
        Self {
            world_params: load_default_world_params(),
        }
    }
}

/// Registers the simulation core: resources, states, messages, and systems.
///
/// Presentation and input plugins that read authoritative state must be added
/// after this plugin has registered the core resources and systems.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimStartupConfig>()
            .init_resource::<NetworkIdAllocator>()
            .init_resource::<EntityLookup>()
            .init_resource::<CommandQueue>()
            .init_resource::<TurnState>()
            .init_state::<TurnPhase>()
            // Buffered messages — written by the processor, consumed by
            // presentation and input tiers.
            .add_message::<EndTurnRequested>()
            .add_message::<CommandRejected>()
            .add_message::<UnitMoved>()
            .add_message::<CityFounded>()
            // Build authoritative terrain and runtime state before
            // presentation-owned OnEnter schedules consume them.
            .add_systems(
                Startup,
                (
                    sim_startup,
                    sim_init.after(sim_startup),
                    units::spawn_settlers.after(sim_init),
                ),
            )
            // Reset movement points at the start of each player-input phase.
            .add_systems(
                OnEnter(TurnPhase::PlayerInput),
                units::reset_movement_points,
            )
            // Player-input phase: process queued commands immediately and
            // listen for End Turn requests.
            .add_systems(
                Update,
                (processor::process_commands, turn_phase::handle_end_turn)
                    .run_if(in_state(TurnPhase::PlayerInput)),
            )
            // Resolution phase: advance the turn counter then return to PlayerInput.
            // This triggers OnEnter(PlayerInput) which resets unit movement points.
            .add_systems(
                Update,
                turn_phase::resolve_turn.run_if(in_state(TurnPhase::Resolution)),
            );
    }
}

/// Generates the world and inserts [`StaticTerrain`].
///
/// Runs in `Startup` before runtime sim state and renderer-owned `OnEnter`
/// systems need [`StaticTerrain`].
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
fn sim_startup(mut commands: Commands, config: Res<SimStartupConfig>) {
    let world =
        generate_world(&config.world_params).expect("world generation must succeed at startup");
    let terrain = terrain::from_world_data(world);
    commands.insert_resource(terrain);
}

/// Initialises remaining simulation resources once [`StaticTerrain`] is ready.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
fn sim_init(mut commands: Commands, terrain: Res<StaticTerrain>) {
    let bounds = terrain.bounds;
    let tile_state = TileState::new(bounds);

    let yield_table = yield_table::load_yield_table(&terrain.biome_registry)
        .expect("yield table must load at startup");

    // Gate 4/4.5 debug query: confirm tile_yield combines static terrain and
    // dynamic tile state at a real start-position coordinate.
    if let Some(&start) = terrain.start_positions.first() {
        let y = tile_yield(start, &terrain, &tile_state, &yield_table);
        info!(
            coord = ?start,
            food = y.food,
            production = y.production,
            gold = y.gold,
            "start-position tile yield"
        );
    }

    commands.insert_resource(tile_state);
    commands.insert_resource(yield_table);

    info!(
        width = bounds.width(),
        height = bounds.height(),
        "sim: tile state and yield table initialised"
    );
}

fn load_default_world_params() -> WorldParams {
    let text = read_workspace_file(DEFAULT_WORLD_PARAMS_PATH);
    let params: WorldParams = ron::from_str(&text).unwrap_or_else(|error| {
        panic!("failed to parse default world params '{DEFAULT_WORLD_PARAMS_PATH}': {error}");
    });
    params.validate().unwrap_or_else(|error| {
        panic!("default world params '{DEFAULT_WORLD_PARAMS_PATH}' failed validation: {error}");
    });
    params
}

fn read_workspace_file(path: &str) -> String {
    let relative_path = Path::new(path);
    std::fs::read_to_string(relative_path).unwrap_or_else(|relative_error| {
        let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative_path);
        std::fs::read_to_string(&workspace_path).unwrap_or_else(|workspace_error| {
            panic!(
                "failed to read '{path}' from current directory ({relative_error}) or workspace path '{}' ({workspace_error})",
                workspace_path.display()
            );
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_startup_config_loads_default_params_asset() {
        let config = SimStartupConfig::default();
        let asset_text = read_workspace_file(DEFAULT_WORLD_PARAMS_PATH);
        let asset_params: WorldParams =
            ron::from_str(&asset_text).expect("default world params asset must parse");

        assert_eq!(config.world_params, asset_params);
    }
}
