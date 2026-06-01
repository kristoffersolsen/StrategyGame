//! Command processor: the only site that mutates authoritative game state.
//!
//! [`process_commands`] is a Bevy system that runs in [`TurnPhase::PlayerInput`].
//! It drains the [`CommandQueue`] every frame, validates each command against
//! current game state, applies the state change immediately, and emits result
//! messages. Invalid commands are rejected without any state change.
//!
//! Turn advancement (MP reset) is separate: it happens when the player presses
//! End Turn (Enter/Space), which triggers `Resolution` → `PlayerInput` via
//! [`crate::sim::turn_phase::resolve_turn`].
//!
//! Validation and mutation use pure helper functions where possible so they
//! can be exercised by unit tests without a full [`bevy::prelude::App`].
//!
//! See design rule §13.5: systems outside the processor never mutate game
//! state directly.

use bevy::prelude::{MessageWriter, Query, Res, ResMut};
use coords::Coord;

use crate::sim::{
    commands::{CommandQueue, GameCommand, PlayerId},
    events::{CommandRejected, RejectionReason, UnitMoved},
    network_id::{EntityLookup, NetworkId},
    terrain::StaticTerrain,
    units::{MovementPoints, UnitCoord, UnitPlayerId, path_cost},
    world_model::TileState,
};

/// Drains the command queue and immediately applies each valid command.
///
/// Runs every frame in [`TurnPhase::PlayerInput`] so that commands issued by
/// the input tier take effect without delay. For each command:
/// - if valid: mutates tile state and unit components, emits a result message
/// - if invalid: emits [`CommandRejected`] without changing state
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub(crate) fn process_commands(
    mut queue: ResMut<CommandQueue>,
    terrain: Res<StaticTerrain>,
    mut tile_state: ResMut<TileState>,
    entity_lookup: Res<EntityLookup>,
    mut rejected: MessageWriter<CommandRejected>,
    mut unit_moved: MessageWriter<UnitMoved>,
    mut units: Query<(&UnitPlayerId, &mut UnitCoord, &mut MovementPoints)>,
) {
    while let Some(cmd) = queue.pop() {
        match cmd {
            GameCommand::MoveUnit {
                player_id,
                unit_id,
                destination,
            } => {
                match apply_move_unit(
                    player_id,
                    unit_id,
                    destination,
                    &terrain,
                    &mut tile_state,
                    &entity_lookup,
                    &mut units,
                ) {
                    Ok(event) => {
                        unit_moved.write(event);
                    }
                    Err(reason) => {
                        rejected.write(CommandRejected { reason });
                    }
                }
            }
        }
    }
}

/// Validates and applies a [`GameCommand::MoveUnit`] command.
///
/// # Errors
///
/// Returns a [`RejectionReason`] describing the validation failure. No state
/// is mutated when an error is returned.
fn apply_move_unit(
    player_id: PlayerId,
    unit_id: NetworkId,
    destination: Coord,
    terrain: &StaticTerrain,
    tile_state: &mut TileState,
    entity_lookup: &EntityLookup,
    units: &mut Query<(&UnitPlayerId, &mut UnitCoord, &mut MovementPoints)>,
) -> Result<UnitMoved, RejectionReason> {
    let entity = entity_lookup
        .get(unit_id)
        .ok_or(RejectionReason::UnitNotFound)?;

    let (unit_player_id, mut unit_coord, mut movement_points) = units
        .get_mut(entity)
        .map_err(|_| RejectionReason::UnitNotFound)?;

    if unit_player_id.0 != player_id {
        return Err(RejectionReason::NotYourUnit);
    }

    let cost = path_cost(unit_coord.0, destination, terrain, tile_state)
        .ok_or(RejectionReason::DestinationImpassable)?;

    if cost > movement_points.remaining {
        return Err(RejectionReason::InsufficientMovementPoints);
    }

    if tile_state
        .tile_occupant
        .get(destination)
        .copied()
        .flatten()
        .is_some()
    {
        return Err(RejectionReason::DestinationOccupied);
    }

    let from = unit_coord.0;

    tile_state.tile_occupant.set(from, None);
    tile_state.tile_occupant.set(destination, Some(unit_id));
    unit_coord.0 = destination;
    movement_points.remaining = movement_points.remaining.saturating_sub(cost);

    Ok(UnitMoved {
        unit_id,
        player_id,
        from,
        to: destination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use content::Registry;
    use coords::MapBounds;
    use worldgen::{BiomeRegistry, Field, ResourceRegistry};

    use crate::sim::{
        terrain::StaticTerrain,
        units::{path_cost, reachable_tiles},
        world_model::TileState,
    };

    fn passable_terrain(bounds: MapBounds) -> StaticTerrain {
        let biome_registry = BiomeRegistry {
            inner: Registry::build([] as [&str; 0]),
            is_forest: vec![],
        };
        let resource_registry = ResourceRegistry {
            inner: Registry::build([] as [&str; 0]),
            category: vec![],
        };
        StaticTerrain {
            bounds,
            mountain_level: 5,
            height_level: Field::filled(bounds, 2),
            is_hilly: Field::filled(bounds, false),
            has_river: Field::filled(bounds, false),
            is_floodplain: Field::filled(bounds, false),
            is_lake: Field::filled(bounds, false),
            biome: Field::filled(bounds, None),
            forest_density: Field::filled(bounds, 0.0),
            resource: Field::filled(bounds, None),
            movement_cost: Field::filled(bounds, 1),
            start_positions: vec![],
            biome_registry,
            resource_registry,
        }
    }

    #[test]
    fn path_cost_adjacent_is_one() {
        let bounds = MapBounds::new(8, 8);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(3, 3);
        let to = Coord::new(4, 3);
        assert_eq!(
            path_cost(from, to, &terrain, &tile_state),
            Some(1),
            "adjacent passable tile must cost 1"
        );
    }

    #[test]
    fn path_cost_impassable_destination() {
        use crate::sim::terrain::IMPASSABLE;
        let bounds = MapBounds::new(8, 8);
        let mut terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        terrain.movement_cost.set(Coord::new(4, 3), IMPASSABLE);
        assert_eq!(
            path_cost(Coord::new(3, 3), Coord::new(4, 3), &terrain, &tile_state),
            None,
        );
    }

    #[test]
    fn reachable_tiles_origin_has_cost_zero() {
        let bounds = MapBounds::new(8, 8);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(4, 4);
        let result = reachable_tiles(from, 2, &terrain, &tile_state);
        assert_eq!(result.get(&from), Some(&0));
    }

    #[test]
    fn reachable_tiles_respects_max_cost() {
        let bounds = MapBounds::new(10, 10);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(5, 5);
        let result = reachable_tiles(from, 1, &terrain, &tile_state);
        for &cost in result.values() {
            assert!(cost <= 1);
        }
    }

    #[test]
    fn identical_command_sequences_produce_identical_reachable_sets() {
        let bounds = MapBounds::new(10, 10);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(5, 5);

        let result_a = reachable_tiles(from, 3, &terrain, &tile_state);
        let result_b = reachable_tiles(from, 3, &terrain, &tile_state);

        let mut keys_a: Vec<Coord> = result_a.keys().copied().collect();
        let mut keys_b: Vec<Coord> = result_b.keys().copied().collect();
        keys_a.sort_by_key(|c| (c.q, c.r));
        keys_b.sort_by_key(|c| (c.q, c.r));
        assert_eq!(
            keys_a, keys_b,
            "identical inputs must produce identical reachable sets"
        );
    }
}
