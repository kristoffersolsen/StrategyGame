//! Determinism integration tests for the world-generation pipeline.
//!
//! These tests verify that generation is a pure function of its inputs:
//! the same `WorldParams` always produces byte-identical output, across
//! runs, across machines, and across rebuilds. This property is
//! foundational for multiplayer correctness (design §0.4).
//!
//! # Structure
//!
//! Tests are split into three groups:
//!
//! 1. **Same-seed equality** — two runs with identical params produce
//!    identical output. These run on every `cargo test` invocation and
//!    catch regressions immediately. Hashes the generated tectonics
//!    and climate fields, so the output genuinely varies by seed.
//!
//! 2. **Cross-seed inequality** — different seeds produce different
//!    output. Sanity check against a collapsed RNG.
//!
//! 3. **Reference hash** — a known `(seed, params)` pair produces a
//!    specific committed blake3 hash. This catches intentional or
//!    accidental generation-output drift.
//!
//! # How to update the reference hash
//!
//! After a deliberate generation change, run:
//!
//! ```text
//! cargo test -p worldgen reference_hash -- --nocapture
//! ```
//!
//! The test prints the new hash. Paste it into `REFERENCE_HASH` below
//! and commit alongside the generation change. The commit message must
//! note that the reference hash changed and why.

use blake3::Hasher;
use worldgen::{
    BasicParams, BiomeHandle, BoundaryType, Direction, Field, MapSize, MineralCharacter, PlateType,
    ResourceHandle, RiverSize, WorldParams,
    debug_viz::{Grayscale, dump_field_pixels},
    generate_world,
};

// ------------- helpers -------------

/// Builds a minimal but realistic `WorldParams` for determinism tests.
///
/// Uses `Small` map size so tests run fast. Seed is the caller's choice.
fn test_params(seed: u64) -> WorldParams {
    WorldParams {
        seed,
        basic: BasicParams {
            map_size: MapSize::Small,
            ..BasicParams::default()
        },
        ..WorldParams::default()
    }
}

/// Hashes `WorldData` directly via blake3.
fn world_data_hash(params: &WorldParams) -> blake3::Hash {
    let world = generate_world(params).expect("generate_world failed");
    let mut hasher = Hasher::new();
    hasher.update(b"WorldData:v6");
    hasher.update(&world.bounds.width().to_le_bytes());
    hasher.update(&world.bounds.height().to_le_bytes());

    update_len(&mut hasher, world.tectonics.seeds.len());
    for seed in &world.tectonics.seeds {
        update_i32(&mut hasher, seed.q);
        update_i32(&mut hasher, seed.r);
    }
    hasher.update(world.tectonics.plate_id.as_slice());
    update_len(&mut hasher, world.tectonics.plates.len());
    for plate in &world.tectonics.plates {
        update_u8(&mut hasher, plate_type_tag(plate.plate_type));
        update_f32(&mut hasher, plate.motion.dx);
        update_f32(&mut hasher, plate.motion.dy);
        update_u8(&mut hasher, mineral_tag(plate.mineral_character));
    }
    for &boundary in world.tectonics.boundary_type.as_slice() {
        update_u8(&mut hasher, boundary_tag(boundary));
    }
    update_f32_field(&mut hasher, &world.tectonics.elevation);
    hasher.update(world.tectonics.height_level.as_slice());
    for &is_hilly in world.tectonics.is_hilly.as_slice() {
        update_u8(&mut hasher, u8::from(is_hilly));
    }

    update_f32_field(&mut hasher, &world.climate.temperature);
    update_f32_field(&mut hasher, &world.climate.moisture);
    for &[east, south] in world.climate.wind.as_slice() {
        update_f32(&mut hasher, east);
        update_f32(&mut hasher, south);
    }
    update_f32_field(&mut hasher, &world.climate.precipitation);

    for &direction in world.hydrology.flow_direction.as_slice() {
        update_u8(&mut hasher, direction_tag(direction));
    }
    update_f32_field(&mut hasher, &world.hydrology.flow_accumulation);
    for &is_lake in world.hydrology.is_lake.as_slice() {
        update_u8(&mut hasher, u8::from(is_lake));
    }
    for &river in world.hydrology.river_size.as_slice() {
        update_u8(&mut hasher, river_tag(river));
    }
    for &is_floodplain in world.hydrology.is_floodplain.as_slice() {
        update_u8(&mut hasher, u8::from(is_floodplain));
    }
    for &is_canyon in world.hydrology.is_canyon.as_slice() {
        update_u8(&mut hasher, u8::from(is_canyon));
    }
    for &is_canyon_wall in world.hydrology.is_canyon_wall.as_slice() {
        update_u8(&mut hasher, u8::from(is_canyon_wall));
    }
    hasher.update(world.hydrology.height_level.as_slice());

    for &biome in world.biomes.biome.as_slice() {
        update_u8(&mut hasher, biome_tag(biome));
    }
    update_f32_field(&mut hasher, &world.biomes.forest_density);

    for &resource in world.resources.resource.as_slice() {
        update_u8(&mut hasher, resource_tag(resource));
    }

    hasher.finalize()
}

fn update_len(hasher: &mut Hasher, len: usize) {
    let len = u64::try_from(len).expect("field length fits in u64");
    hasher.update(&len.to_le_bytes());
}

fn update_i32(hasher: &mut Hasher, value: i32) {
    hasher.update(&value.to_le_bytes());
}

fn update_u8(hasher: &mut Hasher, value: u8) {
    hasher.update(&[value]);
}

fn update_f32(hasher: &mut Hasher, value: f32) {
    hasher.update(&value.to_bits().to_le_bytes());
}

fn update_f32_field(hasher: &mut Hasher, field: &Field<f32>) {
    for &value in field.as_slice() {
        update_f32(hasher, value);
    }
}

fn plate_type_tag(plate_type: PlateType) -> u8 {
    match plate_type {
        PlateType::Continental => 0,
        PlateType::Oceanic => 1,
    }
}

fn mineral_tag(mineral: MineralCharacter) -> u8 {
    match mineral {
        MineralCharacter::Ferrous => 0,
        MineralCharacter::Precious => 1,
        MineralCharacter::Gems => 2,
        MineralCharacter::Hydrocarbons => 3,
        MineralCharacter::Industrial => 4,
    }
}

fn boundary_tag(boundary: Option<BoundaryType>) -> u8 {
    match boundary {
        None => 0,
        Some(BoundaryType::Transform) => 1,
        Some(BoundaryType::Divergent) => 2,
        Some(BoundaryType::Convergent) => 3,
    }
}

fn direction_tag(direction: Option<Direction>) -> u8 {
    match direction {
        None => 0,
        Some(Direction::East) => 1,
        Some(Direction::SouthEast) => 2,
        Some(Direction::SouthWest) => 3,
        Some(Direction::West) => 4,
        Some(Direction::NorthWest) => 5,
        Some(Direction::NorthEast) => 6,
    }
}

fn river_tag(river: Option<RiverSize>) -> u8 {
    match river {
        None => 0,
        Some(RiverSize::Creek) => 1,
        Some(RiverSize::River) => 2,
        Some(RiverSize::MajorRiver) => 3,
    }
}

fn biome_tag(biome: Option<BiomeHandle>) -> u8 {
    match biome {
        None => 0,
        Some(handle) => u8::try_from(handle.index() + 1).expect("biome handle index fits in u8"),
    }
}

fn resource_tag(resource: Option<ResourceHandle>) -> u8 {
    match resource {
        None => 0,
        Some(handle) => u8::try_from(handle.index() + 1).expect("resource handle index fits in u8"),
    }
}

// ------------- Same-seed equality -------------

mod same_seed {
    use super::*;

    #[test]
    fn identical_params_produce_identical_world_data_hash() {
        let params = test_params(0xDEAD_BEEF);
        let hash_a = world_data_hash(&params);
        let hash_b = world_data_hash(&params);
        assert_eq!(
            hash_a, hash_b,
            "same params produced different WorldData hashes across two runs"
        );
    }

    #[test]
    fn repeated_world_generation_is_bit_identical() {
        let params = test_params(0xDEAD_BEEF);
        let hash_a = world_data_hash(&params);
        let hash_b = world_data_hash(&params);
        assert_eq!(
            hash_a, hash_b,
            "same params produced different WorldData hashes across two runs"
        );
    }

    #[test]
    fn determinism_holds_across_multiple_seeds() {
        // Verify the property for a range of seeds, not just one.
        for seed in [0, 1, 42, 0xFFFF_FFFF, u64::MAX] {
            let params = test_params(seed);
            let hash_a = world_data_hash(&params);
            let hash_b = world_data_hash(&params);
            assert_eq!(hash_a, hash_b, "determinism failed for seed {seed:#x}");
        }
    }

    #[test]
    fn determinism_holds_across_map_sizes() {
        // Each map size produces a consistent result on repeated runs.
        for size in [MapSize::Small, MapSize::Standard] {
            let params = WorldParams {
                seed: 99,
                basic: BasicParams {
                    map_size: size,
                    ..BasicParams::default()
                },
                ..WorldParams::default()
            };
            let hash_a = world_data_hash(&params);
            let hash_b = world_data_hash(&params);
            assert_eq!(hash_a, hash_b, "determinism failed for size {size:?}");
        }
    }
}

// ------------- Hydrology invariants -------------

mod hydrology {
    use coords::Coord;

    use super::*;

    #[test]
    fn every_flow_path_terminates_at_water_or_lake_for_seeds_one_through_eight() {
        for seed in 1..=8 {
            let params = test_params(seed);
            let world = generate_world(&params).expect("generate_world failed");
            for (coord, &height_level) in world.hydrology.height_level.iter_coords() {
                if height_level <= 1 || world.hydrology.is_lake[coord] {
                    continue;
                }
                assert_flow_path_terminates(coord, &world, seed);
            }
        }
    }

    fn assert_flow_path_terminates(coord: Coord, world: &worldgen::WorldData, seed: u64) {
        let mut current = coord;
        for _ in 0..world.hydrology.flow_direction.len() {
            if world.hydrology.height_level[current] <= 1 || world.hydrology.is_lake[current] {
                return;
            }
            let Some(direction) = world.hydrology.flow_direction[current] else {
                panic!("flow path for seed {seed} reached non-lake land terminal at {current}");
            };
            current = world
                .bounds
                .neighbor(current, direction)
                .expect("flow direction must stay in bounds");
        }
        panic!("flow path for seed {seed} starting at {coord} did not terminate");
    }
}

// ------------- Cross-seed inequality -------------

mod cross_seed {
    use super::*;

    #[test]
    fn different_seeds_produce_different_world_data_hashes() {
        // If all seeds collapse to the same output, the RNG is broken.
        let hash_0 = world_data_hash(&test_params(0));
        let hash_1 = world_data_hash(&test_params(1));
        assert_ne!(
            hash_0, hash_1,
            "seeds 0 and 1 produced identical WorldData hashes — RNG may be broken"
        );
    }

    #[test]
    fn different_map_sizes_produce_different_world_data_hashes() {
        let small = WorldParams {
            seed: 42,
            basic: BasicParams {
                map_size: MapSize::Small,
                ..BasicParams::default()
            },
            ..WorldParams::default()
        };
        let standard = WorldParams {
            seed: 42,
            basic: BasicParams {
                map_size: MapSize::Standard,
                ..BasicParams::default()
            },
            ..WorldParams::default()
        };
        let hash_small = world_data_hash(&small);
        let hash_standard = world_data_hash(&standard);
        assert_ne!(
            hash_small, hash_standard,
            "different map sizes produced identical WorldData hashes"
        );
    }
}

// ------------- Reference hash -------------

mod reference_hash {
    use super::*;

    /// blake3 hash of the generated `WorldData` for
    /// seed `0xDEAD_BEEF` on a `Small` map.
    ///
    /// This value must be updated whenever generation output changes
    /// intentionally. Version v6: converted Biome/ResourceId enums to
    /// registry handles (Stage 3.7). The hash value is stable because
    /// handle assignment is alphabetical and deterministic.
    const REFERENCE_WORLD_DATA_HASH: &str =
        "82e257a5932ead19e41afdb18129e8f705ce24c7e09ae0483aada85099cba45b";

    #[test]
    fn world_data_matches_committed_reference_hash() {
        let params = test_params(0xDEAD_BEEF);
        let hash = world_data_hash(&params);
        let hash_hex = hash.to_hex();

        println!("current WorldData hash: {hash_hex}");
        println!("expected:               {REFERENCE_WORLD_DATA_HASH}");

        assert_eq!(
            hash_hex.as_str(),
            REFERENCE_WORLD_DATA_HASH,
            "WorldData hash changed — if intentional, update \
             REFERENCE_WORLD_DATA_HASH and commit alongside the change"
        );
    }
}

// ------------- Pixel dump visual validation -------------

mod pixel_dump {
    use super::*;

    /// Verifies the pixel dump produces an image with the correct
    /// dimensions for each map size. This catches regressions where
    /// `MapBounds` and the pixel renderer fall out of sync.
    #[test]
    fn pixel_dump_dimensions_match_map_size() {
        for (size, expected_w, expected_h) in [
            (MapSize::Small, 120_u32, 80_u32),
            (MapSize::Standard, 180, 120),
        ] {
            let params = WorldParams {
                seed: 0,
                basic: BasicParams {
                    map_size: size,
                    ..BasicParams::default()
                },
                ..WorldParams::default()
            };
            let world = generate_world(&params).expect("generate_world failed");
            let bounds = world.bounds;

            #[allow(clippy::cast_precision_loss)]
            let field = Field::from_fn(bounds, |coord| {
                coord.r as f32 / (bounds.height() - 1) as f32
            });

            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("out.png");
            dump_field_pixels(&field, &path, |v| v, &Grayscale).expect("dump failed");

            let img = image::open(&path).expect("open PNG").into_rgb8();
            assert_eq!(img.width(), expected_w, "width mismatch for {size:?}");
            assert_eq!(img.height(), expected_h, "height mismatch for {size:?}");
        }
    }
}
