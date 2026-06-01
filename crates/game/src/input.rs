//! Player input plugin — selection and movement orders.
//!
//! This module is the input tier: it observes authoritative state,
//! computes derived overlays, and pushes [`GameCommand`]s onto the
//! [`CommandQueue`]. It never mutates game state directly.
//!
//! - Left-click: select a unit whose tile the cursor is on.
//! - Right-click drag: show the reachable-tile overlay for the selected unit.
//! - Right-click release over a reachable tile: issue [`GameCommand::MoveUnit`].

use std::collections::HashMap;

use bevy::{
    input::mouse::MouseButton,
    prelude::{
        App, ButtonInput, Camera, GlobalTransform, IntoScheduleConfigs, KeyCode, MessageWriter,
        Plugin, Query, Res, ResMut, Resource, Update, Window, With, in_state,
    },
    window::PrimaryWindow,
};
use coords::{Coord, MapBounds};

use bevy::prelude::Entity;

use crate::{
    rendering::HEX_SIZE,
    sim::{
        CommandQueue, EndTurnRequested, EntityLookup, GameCommand, PlayerId, StaticTerrain,
        TileState, TurnPhase, UnitCoord, UnitNetworkId,
        units::{MovementPoints, UnitKind, reachable_tiles},
    },
};

/// Z coordinate of the land surface plane — intersection target for ray casts.
const LAND_Z: f32 = 4.0;

/// The input plugin: registers selection and movement-input systems.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionState>()
            .init_resource::<MovementOverlay>()
            .add_systems(
                Update,
                (
                    handle_selection,
                    handle_movement_input,
                    handle_found_city,
                    handle_end_turn_key,
                )
                    .run_if(in_state(TurnPhase::PlayerInput)),
            );
    }
}

/// Tracks which unit entity is currently selected.
#[derive(Resource, Default)]
pub(crate) struct SelectionState {
    /// The currently selected unit entity, if any.
    pub(crate) selected_unit: Option<Entity>,
}

/// Stores the set of tiles the selected unit can reach this turn.
///
/// Computed on right-click press; cleared on right-click release.
#[derive(Resource, Default)]
pub(crate) struct MovementOverlay {
    /// Reachable tiles and their minimum cost from the unit's current position.
    pub(crate) reachable: HashMap<Coord, u32>,
    /// Whether the overlay meshes have been spawned for this overlay set.
    // Stage 6: used by the rendering system to avoid re-spawning identical overlays.
    #[allow(dead_code)]
    pub(crate) spawned: bool,
}

/// Converts a world-space (x, y) position into the nearest hex [`Coord`].
///
/// Uses the inverse of the pointy-top hex center formula from [`hex_center`].
/// Returns `None` if the computed coordinate falls outside `bounds`.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn world_to_coord(x: f32, y: f32, bounds: MapBounds) -> Option<Coord> {
    let sqrt3 = f32::sqrt(3.0);

    let r_float = (-y / HEX_SIZE - 1.0) / 1.5;
    let wrapped_x = (x - HEX_SIZE * sqrt3 * 0.5) / (HEX_SIZE * sqrt3);
    let q_float = wrapped_x - 0.5 * r_float;

    // Cube-coordinate rounding: round all three cube axes and fix the one
    // with the largest rounding error to maintain the cube constraint q+r+s=0.
    let s_float = -q_float - r_float;

    let q_round = q_float.round();
    let r_round = r_float.round();
    let s_round = s_float.round();

    let q_diff = (q_round - q_float).abs();
    let r_diff = (r_round - r_float).abs();
    let s_diff = (s_round - s_float).abs();

    let (q, r) = if q_diff > r_diff && q_diff > s_diff {
        let q_fixed = -r_round - s_round;
        (q_fixed as i32, r_round as i32)
    } else if r_diff > s_diff {
        let r_fixed = -q_round - s_round;
        (q_round as i32, r_fixed as i32)
    } else {
        (q_round as i32, r_round as i32)
    };

    bounds.normalize(Coord::new(q, r))
}

/// Casts a ray from the cursor into the scene and returns the hex tile it
/// intersects, or `None` if there is no primary window or the cursor is
/// outside the map.
pub(crate) fn cursor_to_coord(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    terrain: &StaticTerrain,
) -> Option<Coord> {
    let cursor_pos = window.cursor_position()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_pos)
        .ok()?;

    // Intersect the ray with the land-height plane (z = LAND_Z).
    let t = (LAND_Z - ray.origin.z) / ray.direction.z;
    if t < 0.0 {
        return None;
    }
    let world_pos = ray.origin + ray.direction * t;
    world_to_coord(world_pos.x, world_pos.y, terrain.bounds)
}

/// Handles left-click selection: selects the unit at the clicked tile, or
/// clears the selection if the tile is empty.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
fn handle_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    terrain: Res<StaticTerrain>,
    tile_state: Res<TileState>,
    entity_lookup: Res<EntityLookup>,
    mut selection: ResMut<SelectionState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };

    let coord = cursor_to_coord(window, camera, camera_transform, &terrain);

    selection.selected_unit = coord.and_then(|coord| {
        let occupant_id = tile_state.tile_occupant.get(coord).copied().flatten()?;
        entity_lookup.get(occupant_id)
    });
}

/// Handles right-click movement input for the selected unit.
///
/// - On right-click press: computes the reachable overlay.
/// - On right-click release: issues a [`GameCommand::MoveUnit`] if the cursor
///   is over a reachable tile, then clears the overlay.
// Bevy system parameters are always passed by value. Nine parameters is
// unavoidable for a Bevy system with this many independent resources.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
fn handle_movement_input(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    terrain: Res<StaticTerrain>,
    tile_state: Res<TileState>,
    selection: Res<SelectionState>,
    units: Query<(&UnitNetworkId, &UnitCoord, &MovementPoints)>,
    mut overlay: ResMut<MovementOverlay>,
    mut queue: ResMut<CommandQueue>,
) {
    let Some(selected_entity) = selection.selected_unit else {
        return;
    };

    let Ok((unit_net_id, unit_coord, movement_points)) = units.get(selected_entity) else {
        return;
    };

    if mouse.just_pressed(MouseButton::Right) {
        let reachable = reachable_tiles(
            unit_coord.0,
            movement_points.remaining,
            &terrain,
            &tile_state,
        );
        overlay.reachable = reachable;
        overlay.spawned = false;
    }

    if mouse.just_released(MouseButton::Right) {
        let Ok(window) = window.single() else {
            overlay.reachable.clear();
            overlay.spawned = false;
            return;
        };
        let Ok((camera, camera_transform)) = camera.single() else {
            overlay.reachable.clear();
            overlay.spawned = false;
            return;
        };

        let coord = cursor_to_coord(window, camera, camera_transform, &terrain);
        if let Some(dest) = coord
            && overlay.reachable.contains_key(&dest)
            && dest != unit_coord.0
        {
            queue.push(GameCommand::MoveUnit {
                player_id: PlayerId(0),
                unit_id: unit_net_id.0,
                destination: dest,
            });
        }

        overlay.reachable.clear();
        overlay.spawned = false;
    }
}

/// Issues a [`GameCommand::FoundCity`] when the player presses `F` with a
/// settler selected.
///
/// The processor validates the command and rejects it with a
/// [`CommandRejected`] message if the tile is not foundable or is too
/// close to an existing city.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
fn handle_found_city(
    keyboard: Res<ButtonInput<KeyCode>>,
    selection: Res<SelectionState>,
    units: Query<(&UnitNetworkId, &UnitKind)>,
    mut queue: ResMut<CommandQueue>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    let Some(selected_entity) = selection.selected_unit else {
        return;
    };
    let Ok((net_id, kind)) = units.get(selected_entity) else {
        return;
    };
    if *kind != UnitKind::Settler {
        return;
    }
    queue.push(GameCommand::FoundCity {
        player_id: PlayerId(0),
        settler_id: net_id.0,
    });
}

/// Writes [`EndTurnRequested`] when the player presses Enter or Space.
///
/// This triggers the resolution cycle: turn counter increments and unit
/// movement points reset.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
fn handle_end_turn_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut end_turn: MessageWriter<EndTurnRequested>,
) {
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        end_turn.write(EndTurnRequested);
    }
}
