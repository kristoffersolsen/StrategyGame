//! Lighting setup for the P3 3D scene.

use bevy::prelude::{
    ClearColor, Color, Commands, DirectionalLight, GlobalAmbientLight, ResMut, Transform, Vec3,
    default,
};

const SHADOWS_ENABLED: bool = false;

pub(crate) fn spawn_lighting(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    ambient.color = Color::WHITE;
    ambient.brightness = 120.0;

    commands.insert_resource(ClearColor(Color::srgb(0.53, 0.70, 0.87)));
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: SHADOWS_ENABLED,
            ..default()
        },
        Transform::from_xyz(-400.0, 400.0, 600.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));
}
