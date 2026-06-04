//! Map camera and FPS overlay.
#![allow(clippy::needless_pass_by_value)]

use std::time::Duration;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::system::SystemParam,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton},
    prelude::{
        BackgroundColor, ButtonInput, Camera, Camera3d, Color, Commands, Component,
        GlobalTransform, KeyCode, Node, PositionType, Projection, Query, Res, ResMut, Resource,
        Text, TextColor, TextFont, Time, Timer, TimerMode, Transform, Val, Vec2, Vec3, With,
        default, px,
    },
    window::PrimaryWindow,
};

use crate::{
    input::cursor_to_coord,
    rendering::{chunk_spawner::MapRenderBounds, hex_mesh::tile_center_position},
    sim::{StaticTerrain, TurnState},
};

const PAN_SPEED: f32 = 500.0;
const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 8.0;
const ZOOM_STEP: f32 = 0.12;
const ORBIT_INITIAL_DISTANCE: f32 = 900.0;
const ORBIT_INITIAL_YAW: f32 = -std::f32::consts::FRAC_PI_2;
const ORBIT_INITIAL_PITCH: f32 = std::f32::consts::PI / 3.5;
const ORBIT_MIN_PITCH: f32 = 0.20;
const ORBIT_MAX_PITCH: f32 = 1.48;
const ORBIT_MIN_DISTANCE: f32 = 80.0;
const ORBIT_MAX_DISTANCE: f32 = 1_200.0;
const ORBIT_MOUSE_SENSITIVITY: f32 = 0.005;
const ORBIT_SCROLL_STEP: f32 = 0.12;
const ORBIT_KEYBOARD_PAN_FACTOR: f32 = 1.0;
const ORBIT_MOUSE_PAN_FACTOR: f32 = 0.0015;

#[derive(Component)]
pub(crate) struct MapCamera;

#[derive(Component)]
pub(crate) struct FpsText;

#[derive(Resource)]
pub(crate) struct FpsOverlayTimer(Timer);

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct CameraOrbit {
    pivot: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

#[derive(SystemParam)]
pub(crate) struct OrbitInput<'w> {
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    mouse_motion: Res<'w, AccumulatedMouseMotion>,
    scroll: Res<'w, AccumulatedMouseScroll>,
    time: Res<'w, Time>,
}

impl Default for FpsOverlayTimer {
    fn default() -> Self {
        Self(Timer::new(Duration::from_millis(500), TimerMode::Repeating))
    }
}

pub(crate) fn spawn_map_camera(
    mut commands: Commands,
    map_bounds: Res<MapRenderBounds>,
    terrain: Res<StaticTerrain>,
) {
    // Center on the settler spawn position; fall back to map centre if there
    // are no start positions (should not happen in practice).
    let pivot = terrain.start_positions.first().map_or_else(
        || Vec3::new(map_bounds.width * 0.5, -map_bounds.height * 0.5, 0.0),
        |&coord| {
            let [x, y, _] = tile_center_position(&terrain, coord);
            Vec3::new(x, y, 0.0)
        },
    );

    let orbit = CameraOrbit {
        pivot,
        distance: ORBIT_INITIAL_DISTANCE,
        yaw: ORBIT_INITIAL_YAW,
        pitch: ORBIT_INITIAL_PITCH,
    };
    let transform = orbit_transform(orbit);
    commands.insert_resource(orbit);
    commands.spawn((MapCamera, Camera3d::default(), transform));
}

pub(crate) fn pan_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    map_bounds: Res<MapRenderBounds>,
    mut camera: Query<(&mut Transform, &Projection), With<MapCamera>>,
) {
    let Ok((mut transform, projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(orthographic) = projection else {
        return;
    };

    let mut direction = bevy::prelude::Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }

    if direction != bevy::prelude::Vec2::ZERO {
        let delta = direction.normalize() * PAN_SPEED * orthographic.scale * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
        clamp_camera(&mut transform, *map_bounds);
    }
}

pub(crate) fn zoom_camera(
    scroll: Res<AccumulatedMouseScroll>,
    map_bounds: Res<MapRenderBounds>,
    mut camera: Query<(&mut Projection, &mut Transform), With<MapCamera>>,
) {
    if scroll.delta.y == 0.0 {
        return;
    }

    let Ok((mut projection, mut transform)) = camera.single_mut() else {
        return;
    };

    if let Projection::Orthographic(orthographic) = &mut *projection {
        let zoom = 1.0 - scroll.delta.y * ZOOM_STEP;
        orthographic.scale = (orthographic.scale * zoom).clamp(ZOOM_MIN, ZOOM_MAX);
        clamp_camera(&mut transform, *map_bounds);
    }
}

pub(crate) fn update_orbit_camera(
    input: OrbitInput,
    map_bounds: Res<MapRenderBounds>,
    orbit: Option<ResMut<CameraOrbit>>,
    mut camera: Query<&mut Transform, With<MapCamera>>,
) {
    let Some(mut orbit) = orbit else {
        return;
    };
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };

    if input.mouse_buttons.pressed(MouseButton::Middle) {
        orbit.yaw -= input.mouse_motion.delta.x * ORBIT_MOUSE_SENSITIVITY;
        orbit.pitch = (orbit.pitch + input.mouse_motion.delta.y * ORBIT_MOUSE_SENSITIVITY)
            .clamp(ORBIT_MIN_PITCH, ORBIT_MAX_PITCH);
    }

    if input.scroll.delta.y != 0.0 {
        let zoom = 1.0 - input.scroll.delta.y * ORBIT_SCROLL_STEP;
        orbit.distance = (orbit.distance * zoom).clamp(ORBIT_MIN_DISTANCE, ORBIT_MAX_DISTANCE);
    }

    let (forward, right) = orbit_ground_axes(orbit.yaw);
    let mut keyboard_pan = Vec2::ZERO;
    if input.keyboard.pressed(KeyCode::KeyA) || input.keyboard.pressed(KeyCode::ArrowLeft) {
        keyboard_pan -= right;
    }
    if input.keyboard.pressed(KeyCode::KeyD) || input.keyboard.pressed(KeyCode::ArrowRight) {
        keyboard_pan += right;
    }
    if input.keyboard.pressed(KeyCode::KeyW) || input.keyboard.pressed(KeyCode::ArrowUp) {
        keyboard_pan += forward;
    }
    if input.keyboard.pressed(KeyCode::KeyS) || input.keyboard.pressed(KeyCode::ArrowDown) {
        keyboard_pan -= forward;
    }
    if keyboard_pan != Vec2::ZERO {
        let delta = keyboard_pan.normalize()
            * orbit.distance
            * ORBIT_KEYBOARD_PAN_FACTOR
            * input.time.delta_secs();
        orbit.pivot.x += delta.x;
        orbit.pivot.y += delta.y;
    }

    if input.mouse_buttons.pressed(MouseButton::Middle) && input.mouse_motion.delta != Vec2::ZERO {
        let delta = (-right * input.mouse_motion.delta.x + forward * input.mouse_motion.delta.y)
            * orbit.distance
            * ORBIT_MOUSE_PAN_FACTOR;
        orbit.pivot.x += delta.x;
        orbit.pivot.y += delta.y;
    }

    clamp_orbit_pivot(&mut orbit, *map_bounds);
    *transform = orbit_transform(*orbit);
}

pub(crate) fn spawn_fps_overlay(mut commands: Commands) {
    commands.spawn((
        Text::new("FPS: --\nFrame: -- ms"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            left: px(8),
            padding: bevy::prelude::UiRect::all(Val::Px(6.0)),
            ..default()
        },
        FpsText,
    ));
}

/// Marker for the turn counter text entity.
#[derive(Component)]
pub(crate) struct TurnText;

/// Spawns the turn counter in the top-right corner.
pub(crate) fn spawn_turn_overlay(mut commands: Commands) {
    commands.spawn((
        Text::new("Turn 0"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            right: px(8),
            padding: bevy::prelude::UiRect::all(Val::Px(6.0)),
            ..default()
        },
        TurnText,
    ));
}

/// Updates the turn counter from [`TurnState`].
pub(crate) fn update_turn_overlay(
    turn_state: Res<TurnState>,
    mut text: Query<&mut Text, With<TurnText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    **text = format!("Turn {}", turn_state.turn);
}

/// Marker for the tile info and coordinate readout text entity.
#[derive(Component)]
pub(crate) struct CoordText;

/// Spawns the tile info and coordinate readout in the bottom-left corner.
pub(crate) fn spawn_coord_overlay(mut commands: Commands) {
    commands.spawn((
        Text::new("Terrain: --\nTile: --"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(8),
            left: px(8),
            padding: bevy::prelude::UiRect::all(Val::Px(6.0)),
            ..default()
        },
        CoordText,
    ));
}

/// Updates the tile info and coordinate readout with the hex under the cursor.
pub(crate) fn update_coord_overlay(
    window: Query<&bevy::window::Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<MapCamera>>,
    terrain: Res<StaticTerrain>,
    mut text: Query<&mut Text, With<CoordText>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };

    **text = match cursor_to_coord(window, camera, camera_transform, &terrain) {
        Some(coord) => format!(
            "{}\nTile: ({}, {})",
            tile_info_label(&terrain, coord),
            coord.q,
            coord.r
        ),
        None => "Terrain: --\nTile: --".to_owned(),
    };
}

fn tile_info_label(terrain: &StaticTerrain, coord: coords::Coord) -> String {
    let biome = terrain.biome.get(coord).copied().flatten();
    let is_forest = biome.is_some_and(|handle| terrain.biome_registry.is_forest_biome(handle));
    let biome_id = biome.map(|handle| terrain.biome_registry.id_of(handle));

    let biome_label = biome_id.map_or_else(
        || terrain_type_label(terrain, coord).to_owned(),
        |id| display_biome_label(id, is_forest),
    );

    let mut features = Vec::new();
    if is_forest {
        features.push("forest".to_owned());
    }
    if terrain.is_hilly.get(coord).copied().unwrap_or(false) {
        features.push("hills".to_owned());
    }
    if terrain.has_river.get(coord).copied().unwrap_or(false) {
        features.push("river".to_owned());
    }
    if terrain.is_floodplain.get(coord).copied().unwrap_or(false) {
        features.push("floodplain".to_owned());
    }
    if let Some(resource) = terrain.resource.get(coord).copied().flatten() {
        features.push(display_id_label(terrain.resource_registry.id_of(resource)));
    }

    if features.is_empty() {
        format!("Terrain: {biome_label}")
    } else {
        format!("Terrain: {}, {}", biome_label, features.join(", "))
    }
}

fn display_biome_label(id: &str, is_forest: bool) -> String {
    let base = if is_forest {
        id.strip_suffix("_forest").unwrap_or(id)
    } else {
        id
    };
    display_id_label(base)
}

fn display_id_label(id: &str) -> String {
    id.replace('_', " ")
}

fn terrain_type_label(terrain: &StaticTerrain, coord: coords::Coord) -> &'static str {
    let height_level = terrain.height_level.get(coord).copied().unwrap_or(0);
    if terrain.is_lake.get(coord).copied().unwrap_or(false) {
        "lake"
    } else if height_level == 0 {
        "ocean"
    } else if height_level == 1 {
        "coast"
    } else if height_level >= terrain.mountain_level {
        "mountain"
    } else {
        "land"
    }
}

pub(crate) fn update_fps_overlay(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut timer: ResMut<FpsOverlayTimer>,
    mut text: Query<&mut Text, With<FpsText>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(bevy::diagnostic::Diagnostic::smoothed);
    let frame_time = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(bevy::diagnostic::Diagnostic::smoothed);

    let Ok(mut text) = text.single_mut() else {
        return;
    };
    **text = format!(
        "FPS: {}\nFrame: {} ms",
        fps.map_or_else(|| "--".to_owned(), |value| format!("{value:.1}")),
        frame_time.map_or_else(|| "--".to_owned(), |value| format!("{:.2}", value * 1000.0))
    );
}

fn clamp_camera(transform: &mut Transform, map_bounds: MapRenderBounds) {
    transform.translation.x = transform.translation.x.clamp(0.0, map_bounds.width);
    transform.translation.y = transform.translation.y.clamp(-map_bounds.height, 0.0);
}

fn clamp_orbit_pivot(orbit: &mut CameraOrbit, map_bounds: MapRenderBounds) {
    orbit.pivot.x = orbit.pivot.x.clamp(0.0, map_bounds.width);
    orbit.pivot.y = orbit.pivot.y.clamp(-map_bounds.height, 0.0);
    orbit.pivot.z = 0.0;
}

fn orbit_transform(orbit: CameraOrbit) -> Transform {
    let offset = Vec3::new(
        orbit.distance * orbit.pitch.cos() * orbit.yaw.cos(),
        orbit.distance * orbit.pitch.cos() * orbit.yaw.sin(),
        orbit.distance * orbit.pitch.sin(),
    );
    Transform::from_translation(orbit.pivot + offset).looking_at(orbit.pivot, Vec3::Z)
}

fn orbit_ground_axes(yaw: f32) -> (Vec2, Vec2) {
    let forward = Vec2::new(-yaw.cos(), -yaw.sin()).normalize();
    let right = Vec2::new(forward.y, -forward.x);
    (forward, right)
}
