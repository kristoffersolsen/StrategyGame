//! Orthographic map camera and FPS overlay.
#![allow(clippy::needless_pass_by_value)]

use std::time::Duration;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::AccumulatedMouseScroll,
    prelude::{
        BackgroundColor, ButtonInput, Camera2d, Color, Commands, Component, KeyCode, Node,
        PositionType, Projection, Query, Res, ResMut, Resource, Text, TextColor, TextFont, Time,
        Timer, TimerMode, Transform, Val, With, default, px,
    },
};

use crate::rendering::chunk_spawner::{GeneratedWorld, MapRenderBounds};

const PAN_SPEED: f32 = 500.0;
const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 8.0;
const ZOOM_STEP: f32 = 0.12;

#[derive(Component)]
pub(crate) struct MapCamera;

#[derive(Component)]
pub(crate) struct FpsText;

#[derive(Resource)]
pub(crate) struct FpsOverlayTimer(Timer);

impl Default for FpsOverlayTimer {
    fn default() -> Self {
        Self(Timer::new(Duration::from_millis(500), TimerMode::Repeating))
    }
}

pub(crate) fn spawn_map_camera(
    mut commands: Commands,
    map_bounds: Res<MapRenderBounds>,
    world: Res<GeneratedWorld>,
) {
    let _tile_count = world.data.bounds.width() * world.data.bounds.height();
    commands.spawn((
        MapCamera,
        Camera2d,
        Transform::from_xyz(map_bounds.width * 0.5, -map_bounds.height * 0.5, 1000.0),
    ));
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
        let zoom = orthographic_scale(projection);
        let delta = direction.normalize() * PAN_SPEED * zoom * time.delta_secs();
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

fn orthographic_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(orthographic) => orthographic.scale,
        Projection::Perspective(_) | Projection::Custom(_) => 1.0,
    }
}

fn clamp_camera(transform: &mut Transform, map_bounds: MapRenderBounds) {
    transform.translation.x = transform.translation.x.clamp(0.0, map_bounds.width);
    transform.translation.y = transform.translation.y.clamp(-map_bounds.height, 0.0);
}
