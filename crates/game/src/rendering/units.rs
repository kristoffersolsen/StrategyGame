//! Unit model loading and position synchronisation.
//!
//! Loads GLB scene assets for each unit type and spawns a `SceneRoot` entity
//! per unit sim entity. Listens for [`UnitMoved`] messages to update model
//! transforms when the processor confirms a move.

use bevy::prelude::{Handle, MessageReader};
use bevy::{
    asset::AssetServer,
    gltf::GltfAssetLabel,
    prelude::{
        Commands, Component, Entity, Query, Res, Resource, Scene, SceneRoot, Transform, Vec3, With,
    },
};

use crate::{
    rendering::{RenderStartupConfig, TerrainRenderer, hex_mesh::tile_center_position},
    sim::{
        StaticTerrain, UnitMoved,
        units::{Unit, UnitCoord, UnitKind, UnitNetworkId},
    },
};

/// Z offset above the tile surface for unit models.
const SETTLER_Z_OFFSET: f32 = 1.0;

/// Uniform scale applied to settler models.
const SETTLER_SCALE: f32 = 5.0;

/// Loaded scene handles for unit model assets.
#[derive(Resource)]
pub(crate) struct UnitAssets {
    /// Scene handle for the settler GLB.
    pub(crate) settler: Handle<Scene>,
}

/// Loads unit GLB assets from disk.
///
/// Only runs when the Terrain3d renderer is active; other renderers do not
/// display unit models.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn load_unit_assets(
    mut commands: Commands,
    config: Res<RenderStartupConfig>,
    asset_server: Res<AssetServer>,
) {
    if config.terrain_renderer != TerrainRenderer::Terrain3d {
        return;
    }
    let settler = asset_server
        .load(GltfAssetLabel::Scene(0).from_asset("glb-assets/units/civilian/settler.glb"));
    commands.insert_resource(UnitAssets { settler });
}

/// Marker component placed on the visual model entity for a unit.
///
/// Carries the [`UnitNetworkId`] so position-sync systems can correlate
/// model entities with sim events by network id.
#[derive(Component)]
pub(crate) struct UnitModel;

/// Spawns a visual model entity for each unit sim entity.
///
/// Runs once after settlers are spawned. Only active in Terrain3d mode;
/// returns early for other renderers.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn spawn_unit_models(
    mut commands: Commands,
    config: Res<RenderStartupConfig>,
    terrain: Res<StaticTerrain>,
    unit_assets: Option<Res<UnitAssets>>,
    units: Query<(Entity, &UnitNetworkId, &UnitCoord, &UnitKind), With<Unit>>,
) {
    if config.terrain_renderer != TerrainRenderer::Terrain3d {
        return;
    }
    let Some(unit_assets) = unit_assets else {
        return;
    };

    for (_entity, net_id, coord, kind) in &units {
        let scene = match kind {
            UnitKind::Settler => unit_assets.settler.clone(),
        };
        let [x, y, z] = tile_center_position(&terrain, coord.0);
        commands.spawn((
            UnitModel,
            UnitNetworkId(net_id.0),
            SceneRoot(scene),
            Transform::from_translation(Vec3::new(x, y, z + SETTLER_Z_OFFSET))
                .with_scale(Vec3::splat(SETTLER_SCALE)),
        ));
    }
}

/// Updates model entity transforms when a [`UnitMoved`] message is received.
///
/// Queries the model entities (tagged with [`UnitModel`]) by their
/// [`UnitNetworkId`] component and updates their [`Transform`] to the new
/// tile center.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn sync_unit_positions(
    mut moved: MessageReader<UnitMoved>,
    terrain: Res<StaticTerrain>,
    mut models: Query<(&UnitNetworkId, &mut Transform), With<UnitModel>>,
) {
    for event in moved.read() {
        for (net_id, mut transform) in &mut models {
            if net_id.0 == event.unit_id {
                let [x, y, z] = tile_center_position(&terrain, event.to);
                transform.translation = Vec3::new(x, y, z + SETTLER_Z_OFFSET);
            }
        }
    }
}
