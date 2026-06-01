//! City model loading and rendering.
//!
//! Loads the city GLB asset and spawns a `SceneRoot` entity for each city
//! as [`CityFounded`] messages are received. The settler model consumed
//! during founding is also despawned here, keeping all presentation cleanup
//! in the view tier.

use bevy::prelude::{Handle, MessageReader};
use bevy::{
    asset::AssetServer,
    gltf::GltfAssetLabel,
    prelude::{
        Commands, Component, Entity, Query, Res, Resource, Scene, SceneRoot, Transform, Vec3, With,
    },
};

use super::units::UnitModel;
use crate::{
    rendering::hex_mesh::tile_center_position,
    sim::{CityFounded, StaticTerrain, cities::CityNetworkId, units::UnitNetworkId},
};

/// Z offset above the tile surface for city models.
const CITY_Z_OFFSET: f32 = 0.5;

/// Uniform scale applied to city models.
const CITY_SCALE: f32 = 5.0;

/// Loaded scene handles for city model assets.
#[derive(Resource)]
pub(crate) struct CityAssets {
    /// Scene handle for the city GLB.
    pub(crate) city: Handle<Scene>,
}

/// Loads city GLB assets from disk.
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn load_city_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let city = asset_server.load(GltfAssetLabel::Scene(0).from_asset("glb-assets/city/city.glb"));
    commands.insert_resource(CityAssets { city });
}

/// Marker component placed on the visual model entity for a city.
///
/// Carries the [`CityNetworkId`] so position-sync systems can correlate
/// model entities with sim state by network id.
#[derive(Component)]
pub(crate) struct CityModel;

/// Reacts to [`CityFounded`] messages: spawns a city model entity and
/// despawns the settler model entity that was consumed during founding.
///
// Bevy system parameters are always passed by value.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn spawn_city_on_founded(
    mut commands: Commands,
    mut founded: MessageReader<CityFounded>,
    terrain: Res<StaticTerrain>,
    city_assets: Option<Res<CityAssets>>,
    settler_models: Query<(Entity, &UnitNetworkId), With<UnitModel>>,
) {
    let Some(city_assets) = city_assets else {
        return;
    };

    for event in founded.read() {
        // Despawn the settler model that was consumed by founding.
        for (model_entity, net_id) in &settler_models {
            if net_id.0 == event.consumed_settler_id {
                commands.entity(model_entity).despawn();
                break;
            }
        }

        // Spawn the city model at the founding tile's centre.
        let [x, y, z] = tile_center_position(&terrain, event.coord);
        commands.spawn((
            CityModel,
            CityNetworkId(event.city_id),
            SceneRoot(city_assets.city.clone()),
            Transform::from_translation(Vec3::new(x, y, z + CITY_Z_OFFSET))
                .with_scale(Vec3::splat(CITY_SCALE)),
        ));
    }
}
