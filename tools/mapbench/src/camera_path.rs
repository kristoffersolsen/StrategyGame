use bevy::prelude::{Transform, Vec3};

/// Yaw and pitch match `CameraOrbit` initial values in `game::rendering::camera`.
const YAW: f32 = -std::f32::consts::FRAC_PI_2;
const PITCH: f32 = std::f32::consts::PI / 3.5;
const INITIAL_DISTANCE: f32 = 900.0;
const MIN_DISTANCE: f32 = 300.0;

/// Returns `(pivot, distance)` for the scripted orbit camera at `elapsed` seconds.
///
/// The four-phase path exercises full map traversal and zoom range:
/// * 0–10 s  — pan east: centre → right edge
/// * 10–20 s — pan west: right edge → left edge
/// * 20–25 s — zoom in: distance 900 → 300
/// * 25–30 s — zoom out: distance 300 → 900
pub(crate) fn scripted_camera(elapsed: f64, map_width: f32, map_height: f32) -> (Vec3, f32) {
    // f64 → f32 is intentional: bench timing is measured in f64 but camera coords are f32.
    #[allow(clippy::cast_possible_truncation)]
    let t = elapsed as f32;
    let center_x = map_width * 0.5;
    let center_y = -map_height * 0.5;

    let pivot_x = if t < 10.0 {
        center_x + (t / 10.0) * center_x
    } else if t < 20.0 {
        map_width * (1.0 - (t - 10.0) / 10.0)
    } else {
        0.0
    };

    let distance = if t < 20.0 {
        INITIAL_DISTANCE
    } else if t < 25.0 {
        let frac = (t - 20.0) / 5.0;
        INITIAL_DISTANCE - frac * (INITIAL_DISTANCE - MIN_DISTANCE)
    } else {
        let frac = (t - 25.0) / 5.0;
        MIN_DISTANCE + frac * (INITIAL_DISTANCE - MIN_DISTANCE)
    };

    let pivot = Vec3::new(pivot_x.clamp(0.0, map_width), center_y, 0.0);
    (pivot, distance)
}

/// Computes the camera `Transform` for a given orbit pivot and distance.
///
/// Replicates `orbit_transform` from `game::rendering::camera` with fixed
/// yaw/pitch so the bench path is independent of user input.
pub(crate) fn orbit_transform(pivot: Vec3, distance: f32) -> Transform {
    let offset = Vec3::new(
        distance * PITCH.cos() * YAW.cos(),
        distance * PITCH.cos() * YAW.sin(),
        distance * PITCH.sin(),
    );
    Transform::from_translation(pivot + offset).looking_at(pivot, Vec3::Z)
}
