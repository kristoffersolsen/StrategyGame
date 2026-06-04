//! Unit components, spawn system, and pathfinding.
//!
//! This module owns the ECS representation of mobile game actors (units).
//! Static world data lives in [`super::terrain::StaticTerrain`]; mutable
//! per-tile state lives in [`super::world_model::TileState`].
//!
//! Pathfinding is Dijkstra over the hex grid using movement costs from
//! [`StaticTerrain::movement_cost`]. Occupied tiles are treated as walls
//! so no two units may share a tile (1UPT).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use bevy::prelude::{Commands, Component, Entity, Res, ResMut};
use coords::Coord;

use crate::sim::{
    commands::PlayerId,
    network_id::{EntityLookup, NetworkId, NetworkIdAllocator},
    terrain::{EdgeClass, edge_class},
    terrain::{IMPASSABLE, StaticTerrain},
    turn_phase::TurnState,
    world_model::TileState,
};

/// Movement points granted to a settler each turn.
pub(crate) const SETTLER_MOVEMENT: u32 = 4;

/// Marker component for unit entities.
#[derive(Component)]
pub(crate) struct Unit;

/// Distinguishes unit types.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitKind {
    /// A settler unit — can found cities and has 2 movement points per turn.
    Settler,
}

/// Wraps the authoritative [`NetworkId`] for a unit entity.
///
/// Used by rendering systems to correlate model entities with simulation
/// entities without relying on unstable [`bevy::prelude::Entity`] indices.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct UnitNetworkId(pub(crate) NetworkId);

/// The [`PlayerId`] that owns this unit.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct UnitPlayerId(pub(crate) PlayerId);

/// Hex-grid position of a unit entity.
///
/// Kept in sync with [`TileState::tile_occupant`] by the command processor.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct UnitCoord(pub(crate) Coord);

/// Movement budget for a unit within a single turn.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct MovementPoints {
    /// Points remaining this turn.
    pub(crate) remaining: u32,
    /// Points restored at the start of each turn.
    pub(crate) max_per_turn: u32,
}

/// Spawns one settler for [`PlayerId(0)`] at the first start position.
///
/// Writes the occupant entry into [`TileState`] and registers the
/// entity in [`EntityLookup`] so the rest of the sim can find it.
///
/// # Panics
///
/// Panics if [`StaticTerrain::start_positions`] is empty — world generation
/// must always produce at least one start position.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn spawn_settlers(
    mut commands: Commands,
    terrain: Res<StaticTerrain>,
    mut tile_state: ResMut<TileState>,
    mut alloc: ResMut<NetworkIdAllocator>,
    mut entity_lookup: ResMut<EntityLookup>,
) {
    let coord = *terrain
        .start_positions
        .first()
        .expect("world generation must provide at least one start position");

    let network_id = alloc.next();
    let player_id = PlayerId(0);

    let entity: Entity = commands
        .spawn((
            Unit,
            UnitKind::Settler,
            UnitNetworkId(network_id),
            UnitPlayerId(player_id),
            UnitCoord(coord),
            MovementPoints {
                remaining: SETTLER_MOVEMENT,
                max_per_turn: SETTLER_MOVEMENT,
            },
        ))
        .id();

    tile_state.tile_occupant.set(coord, Some(network_id));
    entity_lookup.insert(network_id, entity);

    tracing::info!(
        coord = ?coord,
        network_id = ?network_id,
        "spawned settler"
    );
}

/// Resets [`MovementPoints::remaining`] to [`MovementPoints::max_per_turn`]
/// for every unit at the start of each [`TurnPhase::PlayerInput`] phase.
///
/// Skips turn 0 so units do not get reset before the first turn begins.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn reset_movement_points(
    turn_state: Res<TurnState>,
    mut units: bevy::prelude::Query<&mut MovementPoints>,
) {
    if turn_state.turn == 0 {
        return;
    }
    for mut mp in &mut units {
        mp.remaining = mp.max_per_turn;
    }
}

/// Dijkstra BFS from `from` up to `max_cost` movement points.
///
/// Returns a map from reachable [`Coord`] to minimum cost to reach it.
/// Impassable tiles and occupied tiles (other than `from`) are skipped.
/// The origin tile itself is always included with cost `0`.
pub(crate) fn reachable_tiles(
    from: Coord,
    max_cost: u32,
    terrain: &StaticTerrain,
    tile_state: &TileState,
) -> HashMap<Coord, u32> {
    // BinaryHeap<(Reverse<cost>, q, r)> — Coord does not implement Ord so we
    // store the components directly.
    let mut heap: BinaryHeap<(Reverse<u32>, i32, i32)> = BinaryHeap::new();
    let mut best: HashMap<Coord, u32> = HashMap::new();

    best.insert(from, 0);
    heap.push((Reverse(0), from.q, from.r));

    while let Some((Reverse(cost), q, r)) = heap.pop() {
        let current = Coord::new(q, r);

        if cost > *best.get(&current).unwrap_or(&u32::MAX) {
            continue;
        }

        for neighbor in terrain.bounds.neighbors(current).into_iter().flatten() {
            if edge_class(current, neighbor, terrain) == EdgeClass::Cliff {
                continue;
            }

            let move_cost = terrain
                .movement_cost
                .get(neighbor)
                .copied()
                .unwrap_or(IMPASSABLE);

            if move_cost == IMPASSABLE {
                continue;
            }

            // Occupied tiles (by any unit other than the starting tile) block movement.
            if neighbor != from
                && tile_state
                    .tile_occupant
                    .get(neighbor)
                    .copied()
                    .flatten()
                    .is_some()
            {
                continue;
            }

            let new_cost = cost.saturating_add(u32::from(move_cost));
            if new_cost > max_cost {
                continue;
            }

            let entry = best.entry(neighbor).or_insert(u32::MAX);
            if new_cost < *entry {
                *entry = new_cost;
                heap.push((Reverse(new_cost), neighbor.q, neighbor.r));
            }
        }
    }

    best
}

/// Returns the minimum movement-point cost to reach `to` from `from`, or
/// `None` if `to` is unreachable within an arbitrarily large budget.
///
/// Uses [`reachable_tiles`] with [`u32::MAX`] as the cost cap, then looks
/// up `to` in the result.
pub(crate) fn path_cost(
    from: Coord,
    to: Coord,
    terrain: &StaticTerrain,
    tile_state: &TileState,
) -> Option<u32> {
    let reachable = reachable_tiles(from, u32::MAX, terrain, tile_state);
    reachable.get(&to).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use content::Registry;
    use coords::MapBounds;
    use worldgen::{BiomeRegistry, Field, ResourceRegistry};

    use crate::sim::{terrain::StaticTerrain, world_model::TileState};

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
    fn reachable_includes_origin_with_cost_zero() {
        let bounds = MapBounds::new(8, 8);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(4, 4);

        let result = reachable_tiles(from, 2, &terrain, &tile_state);
        assert_eq!(result.get(&from), Some(&0), "origin must be cost 0");
    }

    #[test]
    fn reachable_respects_max_cost() {
        let bounds = MapBounds::new(10, 10);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(5, 5);

        let result = reachable_tiles(from, 1, &terrain, &tile_state);
        for &cost in result.values() {
            assert!(cost <= 1, "no tile should exceed max_cost=1");
        }
        // All immediate neighbors of an interior tile should be reachable at cost 1.
        let neighbors_reachable = bounds
            .neighbors(from)
            .iter()
            .filter_map(|n| *n)
            .filter(|n| result.contains_key(n))
            .count();
        assert_eq!(
            neighbors_reachable, 6,
            "all 6 neighbors should be reachable at cost 1"
        );
    }

    #[test]
    fn impassable_tiles_are_excluded() {
        let bounds = MapBounds::new(8, 8);
        let mut terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(3, 3);
        let blocked = Coord::new(4, 3);
        terrain.movement_cost.set(blocked, IMPASSABLE);

        let result = reachable_tiles(from, 5, &terrain, &tile_state);
        assert!(
            !result.contains_key(&blocked),
            "impassable tile must not appear in reachable set"
        );
    }

    #[test]
    fn cliff_edges_are_excluded() {
        let bounds = MapBounds::new(8, 8);
        let mut terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(3, 3);
        let cliff_neighbor = Coord::new(4, 3);
        terrain.height_level.set(cliff_neighbor, 4);

        let result = reachable_tiles(from, 5, &terrain, &tile_state);
        assert!(
            !result.contains_key(&cliff_neighbor),
            "delta >= 2 edge must not appear in reachable set"
        );
    }

    #[test]
    fn occupied_tiles_are_excluded() {
        let bounds = MapBounds::new(8, 8);
        let terrain = passable_terrain(bounds);
        let mut tile_state = TileState::new(bounds);
        let from = Coord::new(3, 3);
        let occupied = Coord::new(4, 3);

        let mut alloc = crate::sim::network_id::NetworkIdAllocator::default();
        let other_unit = alloc.next();
        tile_state.tile_occupant.set(occupied, Some(other_unit));

        let result = reachable_tiles(from, 5, &terrain, &tile_state);
        assert!(
            !result.contains_key(&occupied),
            "occupied tile must not appear in reachable set"
        );
    }

    #[test]
    fn path_cost_returns_none_for_impassable_destination() {
        let bounds = MapBounds::new(8, 8);
        let mut terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(3, 3);
        let dest = Coord::new(4, 3);
        terrain.movement_cost.set(dest, IMPASSABLE);

        assert_eq!(
            path_cost(from, dest, &terrain, &tile_state),
            None,
            "impassable destination must return None"
        );
    }

    #[test]
    fn path_cost_adjacent_passable_tile() {
        let bounds = MapBounds::new(8, 8);
        let terrain = passable_terrain(bounds);
        let tile_state = TileState::new(bounds);
        let from = Coord::new(3, 3);
        let dest = Coord::new(4, 3);

        let cost = path_cost(from, dest, &terrain, &tile_state);
        assert_eq!(cost, Some(1), "adjacent passable tile must cost 1");
    }

    #[test]
    fn determinism_same_start_same_result() {
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
        assert_eq!(keys_a, keys_b, "reachable_tiles must be deterministic");

        for key in &keys_a {
            assert_eq!(result_a[key], result_b[key]);
        }
    }
}
