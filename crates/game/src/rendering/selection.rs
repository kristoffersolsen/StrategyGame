//! Selection highlight and reachable-tile overlay rendering.
//!
//! Maintains a single persistent `SelectionHighlight` entity that follows
//! the selected unit, and a dynamic set of `ReachableTileOverlay` entities
//! that are spawned/despawned as the movement overlay changes.

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::{
        AlphaMode, Assets, Color, Commands, Component, DetectChanges, Entity, Mesh, Mesh3d,
        MeshMaterial3d, Query, Res, ResMut, StandardMaterial, Transform, Vec3, Visibility, With,
    },
    render::render_resource::PrimitiveTopology,
};

use crate::{
    input::{MovementOverlay, SelectionState},
    rendering::hex_mesh::{HEX_SIZE, tile_center_position},
    sim::{
        StaticTerrain,
        units::{Unit, UnitCoord},
    },
};

/// Z offset above the tile surface for the selection ring.
const HIGHLIGHT_Z_OFFSET: f32 = 0.3;

/// Z offset above the tile surface for reachable-tile overlay hexes.
const OVERLAY_Z_OFFSET: f32 = 0.2;

/// Alpha for the reachable-tile overlay material.
const OVERLAY_ALPHA: f32 = 0.45;

/// Marker component for the persistent selection highlight entity.
#[derive(Component)]
pub(crate) struct SelectionHighlight;

/// Marker component for reachable-tile overlay hex entities.
#[derive(Component)]
pub(crate) struct ReachableTileOverlay;

/// Builds a flat hexagonal mesh with the pointy-top corner angles.
///
/// 7 vertices: centre + 6 corners. 6 triangles fan from the centre.
fn flat_hex_mesh(radius: f32) -> Mesh {
    let corners: [f32; 6] = [
        PI / 6.0,
        PI / 2.0,
        5.0 * PI / 6.0,
        7.0 * PI / 6.0,
        3.0 * PI / 2.0,
        11.0 * PI / 6.0,
    ];

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(7);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(7);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(7);
    let mut indices: Vec<u32> = Vec::with_capacity(18);

    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);

    for angle in corners {
        positions.push([radius * angle.cos(), radius * angle.sin(), 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.5 + 0.5 * angle.cos(), 0.5 - 0.5 * angle.sin()]);
    }

    for corner in 0_u32..6 {
        let a = 1 + corner;
        let b = 1 + (corner + 1) % 6;
        indices.extend_from_slice(&[0, a, b]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Spawns the single persistent `SelectionHighlight` entity in a hidden state.
///
/// Run once on entering `RenderAppState::Running`.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn spawn_selection_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(flat_hex_mesh(HEX_SIZE));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 0.0, 0.9),
        emissive: bevy::prelude::LinearRgba::new(0.8, 0.8, 0.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..Default::default()
    });
    commands.spawn((
        SelectionHighlight,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
    ));
}

/// Moves the selection highlight to the selected unit's tile, or hides it.
///
/// Runs every frame in `Update` when the game is in `Running` state.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn update_selection_highlight(
    selection: Res<SelectionState>,
    terrain: Res<StaticTerrain>,
    units: Query<&UnitCoord, With<Unit>>,
    mut highlight: Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>,
) {
    let Ok((mut transform, mut visibility)) = highlight.single_mut() else {
        return;
    };

    let Some(entity) = selection.selected_unit else {
        *visibility = Visibility::Hidden;
        return;
    };

    let Ok(coord) = units.get(entity) else {
        *visibility = Visibility::Hidden;
        return;
    };

    let [x, y, z] = tile_center_position(&terrain, coord.0);
    transform.translation = Vec3::new(x, y, z + HIGHLIGHT_Z_OFFSET);
    *visibility = Visibility::Inherited;
}

/// Spawns or despawns reachable-tile overlay hexes based on [`MovementOverlay`].
///
/// Despawns all existing overlay entities first, then spawns one per
/// reachable tile when the overlay is non-empty.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn update_reachable_overlay(
    mut commands: Commands,
    overlay: Res<MovementOverlay>,
    terrain: Res<StaticTerrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<Entity, With<ReachableTileOverlay>>,
) {
    if !overlay.is_changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    if overlay.reachable.is_empty() {
        return;
    }

    let mesh = meshes.add(flat_hex_mesh(HEX_SIZE));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.4, 1.0, OVERLAY_ALPHA),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..Default::default()
    });

    for coord in overlay.reachable.keys() {
        let [x, y, z] = tile_center_position(&terrain, *coord);
        commands.spawn((
            ReachableTileOverlay,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(x, y, z + OVERLAY_Z_OFFSET)),
        ));
    }
}
