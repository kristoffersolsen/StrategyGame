//! Forest and resource model loading and placement for the P3 renderer.

use bevy::{
    asset::AssetServer,
    gltf::GltfAssetLabel,
    prelude::{Commands, Handle, Quat, Resource, Scene, SceneRoot, Transform, Vec3},
};
use coords::Coord;
use tracing::info;

use crate::rendering::hex_mesh::{HEX_SIZE, tile_center_position};
use crate::sim::terrain::StaticTerrain;

const FOREST_THRESHOLD: f32 = 0.45;
const TREE_SCALE: f32 = 8.0;
const RESOURCE_SCALE: f32 = 4.0;
const TREE_JITTER: f32 = HEX_SIZE * 0.20;
const TREE_Z_OFFSET: f32 = 0.5;
const RESOURCE_Z_OFFSET: f32 = 1.0;
const FISH_Z: f32 = 1.25;

/// Loaded scene handles for static map feature models.
#[derive(Resource)]
pub(crate) struct FeatureModelAssets {
    tropical_trees: Vec<Handle<Scene>>,
    temperate_trees: Vec<Handle<Scene>>,
    boreal_trees: Vec<Handle<Scene>>,
    iron: Handle<Scene>,
    horses: Handle<Scene>,
    gold: Handle<Scene>,
    gems: Handle<Scene>,
    spices: Handle<Scene>,
    wine: Handle<Scene>,
    wheat: Handle<Scene>,
    fish: Handle<Scene>,
    stone: Handle<Scene>,
}

/// Summary of static feature entities spawned for a generated map.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FeatureSpawnCounts {
    trees: u32,
    resources: u32,
}

impl FeatureModelAssets {
    /// Starts loading all P3 map feature model scenes.
    pub(crate) fn load(asset_server: &AssetServer) -> Self {
        Self {
            tropical_trees: vec![
                load_scene(asset_server, "glb-assets/foliage/tree_tropical_A.glb"),
                load_scene(asset_server, "glb-assets/foliage/tree_tropical_B.glb"),
            ],
            temperate_trees: vec![
                load_scene(asset_server, "glb-assets/foliage/tree_temperate_A.glb"),
                load_scene(asset_server, "glb-assets/foliage/tree_temperate_B.glb"),
                load_scene(asset_server, "glb-assets/foliage/tree_temperate_C.glb"),
            ],
            boreal_trees: vec![
                load_scene(asset_server, "glb-assets/foliage/tree_boreal_A.glb"),
                load_scene(asset_server, "glb-assets/foliage/tree_boreal_B.glb"),
            ],
            iron: load_scene(asset_server, "glb-assets/strategic-resource/iron_A.glb"),
            horses: load_scene(asset_server, "glb-assets/strategic-resource/horses_A.glb"),
            gold: load_scene(asset_server, "glb-assets/luxury-resource/gold.glb"),
            gems: load_scene(asset_server, "glb-assets/luxury-resource/gems.glb"),
            spices: load_scene(asset_server, "glb-assets/luxury-resource/spices.glb"),
            wine: load_scene(asset_server, "glb-assets/luxury-resource/wine.glb"),
            wheat: load_scene(asset_server, "glb-assets/bonus-resource/wheat.glb"),
            fish: load_scene(asset_server, "glb-assets/bonus-resource/fish.glb"),
            stone: load_scene(asset_server, "glb-assets/bonus-resource/stone.glb"),
        }
    }

    /// Returns true when every scene and its recursive dependencies have loaded.
    pub(crate) fn all_loaded(&self, asset_server: &AssetServer) -> bool {
        let mut loaded = true;
        self.for_each_handle(|handle| {
            let Some(state) = asset_server.get_recursive_dependency_load_state(handle) else {
                loaded = false;
                return;
            };
            assert!(
                !state.is_failed(),
                "P3 feature model asset failed to load; see Bevy asset errors above"
            );
            loaded &= state.is_loaded();
        });
        loaded
    }

    fn for_each_handle(&self, mut visit: impl FnMut(&Handle<Scene>)) {
        for handle in self
            .tropical_trees
            .iter()
            .chain(self.temperate_trees.iter())
            .chain(self.boreal_trees.iter())
        {
            visit(handle);
        }
        for handle in [
            &self.iron,
            &self.horses,
            &self.gold,
            &self.gems,
            &self.spices,
            &self.wine,
            &self.wheat,
            &self.fish,
            &self.stone,
        ] {
            visit(handle);
        }
    }

    fn tree_pool(&self, biome_id: &str) -> Option<&[Handle<Scene>]> {
        match biome_id {
            "tropical_rainforest" => Some(&self.tropical_trees),
            "temperate_forest" => Some(&self.temperate_trees),
            "boreal_forest" => Some(&self.boreal_trees),
            _ => None,
        }
    }

    fn resource_model(&self, resource_id: &str) -> Option<&Handle<Scene>> {
        match resource_id {
            "iron" => Some(&self.iron),
            "horses" => Some(&self.horses),
            "gold" => Some(&self.gold),
            "gems" => Some(&self.gems),
            "spices" => Some(&self.spices),
            "wine" => Some(&self.wine),
            "wheat" => Some(&self.wheat),
            "fish" => Some(&self.fish),
            "stone" => Some(&self.stone),
            _ => None,
        }
    }
}

/// Spawns deterministic forest and resource model instances for the generated map.
pub(crate) fn spawn_feature_models(
    commands: &mut Commands,
    assets: &FeatureModelAssets,
    terrain: &StaticTerrain,
) -> FeatureSpawnCounts {
    let mut counts = FeatureSpawnCounts {
        trees: 0,
        resources: 0,
    };

    for (coord, density) in terrain.forest_density.iter_coords() {
        if *density <= FOREST_THRESHOLD {
            continue;
        }
        let Some(biome_handle) = terrain.biome.get(coord).copied().flatten() else {
            continue;
        };
        let biome_id = terrain.biome_registry.id_of(biome_handle);
        let Some(pool) = assets.tree_pool(biome_id) else {
            continue;
        };
        let model = select_handle(pool, coord, 0xA53D_91F2);
        let [x, y, z] = tile_center_position(terrain, coord);
        let jitter = jitter_offset(coord);
        commands.spawn((
            SceneRoot(model.clone()),
            Transform::from_translation(Vec3::new(x + jitter.x, y + jitter.y, z + TREE_Z_OFFSET))
                .with_rotation(model_rotation(coord))
                .with_scale(Vec3::splat(TREE_SCALE)),
        ));
        counts.trees += 1;
    }

    // Resolve the fish handle once so we can check for water placement.
    let fish_handle = terrain.resource_registry.resolve("fish");

    for (coord, resource) in terrain.resource.iter_coords() {
        let Some(resource_handle) = *resource else {
            continue;
        };
        let resource_id = terrain.resource_registry.id_of(resource_handle);
        let Some(model) = assets.resource_model(resource_id) else {
            continue;
        };
        let [x, y, terrain_z] = tile_center_position(terrain, coord);
        let z = if Some(resource_handle) == fish_handle {
            FISH_Z
        } else {
            terrain_z + RESOURCE_Z_OFFSET
        };
        commands.spawn((
            SceneRoot(model.clone()),
            Transform::from_translation(Vec3::new(x, y, z))
                .with_rotation(model_rotation(coord))
                .with_scale(Vec3::splat(RESOURCE_SCALE)),
        ));
        counts.resources += 1;
    }

    info!(
        trees = counts.trees,
        resources = counts.resources,
        "spawned static map feature models"
    );
    counts
}

fn load_scene(asset_server: &AssetServer, path: &'static str) -> Handle<Scene> {
    asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))
}

fn select_handle(pool: &[Handle<Scene>], coord: Coord, salt: u32) -> &Handle<Scene> {
    let index = usize::try_from(tile_hash(coord, salt)).expect("tile hash fits in usize");
    &pool[index % pool.len()]
}

fn jitter_offset(coord: Coord) -> Vec3 {
    Vec3::new(
        signed_unit(tile_hash(coord, 0x4D29_3C11)) * TREE_JITTER,
        signed_unit(tile_hash(coord, 0x7E5A_8B43)) * TREE_JITTER,
        0.0,
    )
}

fn model_rotation(coord: Coord) -> Quat {
    let yaw = signed_unit(tile_hash(coord, 0xD19F_2E77)) * std::f32::consts::PI;
    Quat::from_rotation_z(yaw) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
}

#[allow(clippy::cast_precision_loss)]
fn signed_unit(value: u32) -> f32 {
    let normalized = value as f32 / u32::MAX as f32;
    normalized.mul_add(2.0, -1.0)
}

fn tile_hash(coord: Coord, salt: u32) -> u32 {
    let mut value = u32::from_ne_bytes(coord.q.to_ne_bytes()).wrapping_mul(0x9E37_79B1);
    value ^= u32::from_ne_bytes(coord.r.to_ne_bytes()).wrapping_mul(0x85EB_CA77);
    value ^= salt;
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^ (value >> 16)
}
