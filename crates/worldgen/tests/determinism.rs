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
//!    catch regressions immediately. Uses a field filled from the
//!    Tectonics sub-RNG so the output genuinely varies by seed.
//!
//! 2. **Cross-seed inequality** — different seeds produce different
//!    output. Sanity check against a collapsed RNG.
//!
//! 3. **Reference hash** — a known `(seed, params)` pair produces a
//!    specific committed blake3 hash. These are `#[ignore]`d until
//!    Stage 3.1 (tectonics) lands: the current stub `WorldData` holds
//!    only `MapBounds`, whose hash is trivially constant. Un-ignore and
//!    commit a real hash in the Stage 3.1 PR.
//!
//! # How to update the reference hash
//!
//! After a deliberate generation change, run:
//!
//! ```text
//! cargo test -p worldgen reference_hash -- --ignored --nocapture
//! ```
//!
//! The test prints the new hash. Paste it into `REFERENCE_HASH` below
//! and commit alongside the generation change. The commit message must
//! note that the reference hash changed and why.

use std::path::Path;

use blake3::Hasher;
use rand_core::Rng;
use worldgen::{
    BasicParams, Field, MapSize, WorldParams,
    debug_viz::{Grayscale, Viridis, dump_field_pixels},
    generate_world,
    rng::{MasterRng, StageId},
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

/// Hashes the raw bytes of a PNG file on disk using blake3.
fn hash_png_file(path: &Path) -> blake3::Hash {
    let bytes = std::fs::read(path).expect("failed to read PNG for hashing");
    blake3::hash(&bytes)
}

/// Renders a field filled with values drawn from the Tectonics sub-RNG
/// to a pixel PNG and returns its blake3 hash.
///
/// Unlike the gradient field, this field's values depend directly on the
/// seed, so different seeds produce genuinely different hashes.
fn seeded_field_hash_for(params: &WorldParams) -> blake3::Hash {
    let world = generate_world(params).expect("generate_world failed");
    let bounds = world.bounds;
    let master = MasterRng::new(params.seed);
    let mut rng = master.fork(StageId::Tectonics);

    // Fill a field with u32 values drawn from the RNG, one per tile.
    // Normalized to [0.0, 1.0] for the colormap.
    #[allow(clippy::cast_precision_loss)]
    let field = Field::from_fn(bounds, |_coord| rng.next_u32() as f32 / u32::MAX as f32);

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("seeded.png");
    dump_field_pixels(&field, &path, |v| v, &Viridis).expect("dump failed");
    hash_png_file(&path)
}

/// Hashes `WorldData` directly via blake3, feeding width and height as
/// bytes. Minimal but correct for the current stub that only carries
/// `MapBounds`.
///
/// Extend this function as `WorldData` gains fields in Stage 3.
fn world_data_hash(params: &WorldParams) -> blake3::Hash {
    let world = generate_world(params).expect("generate_world failed");
    let mut hasher = Hasher::new();
    hasher.update(&world.bounds.width().to_le_bytes());
    hasher.update(&world.bounds.height().to_le_bytes());
    hasher.finalize()
}

// ------------- Same-seed equality -------------

mod same_seed {
    use super::*;

    #[test]
    fn identical_params_produce_identical_gradient_hash() {
        let params = test_params(0xDEAD_BEEF);
        let hash_a = seeded_field_hash_for(&params);
        let hash_b = seeded_field_hash_for(&params);
        assert_eq!(
            hash_a, hash_b,
            "same params produced different gradient PNGs across two runs"
        );
    }

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
    fn determinism_holds_across_multiple_seeds() {
        // Verify the property for a range of seeds, not just one.
        for seed in [0, 1, 42, 0xFFFF_FFFF, u64::MAX] {
            let params = test_params(seed);
            let hash_a = seeded_field_hash_for(&params);
            let hash_b = seeded_field_hash_for(&params);
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
            let hash_a = seeded_field_hash_for(&params);
            let hash_b = seeded_field_hash_for(&params);
            assert_eq!(hash_a, hash_b, "determinism failed for size {size:?}");
        }
    }
}

// ------------- Cross-seed inequality -------------

mod cross_seed {
    use super::*;

    #[test]
    fn different_seeds_produce_different_gradient_hashes() {
        // If all seeds collapse to the same output, the RNG is broken.
        let hash_0 = seeded_field_hash_for(&test_params(0));
        let hash_1 = seeded_field_hash_for(&test_params(1));
        assert_ne!(
            hash_0, hash_1,
            "seeds 0 and 1 produced identical gradient PNGs — RNG may be broken"
        );
    }

    #[test]
    fn different_map_sizes_produce_different_gradient_hashes() {
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
        let hash_small = seeded_field_hash_for(&small);
        let hash_standard = seeded_field_hash_for(&standard);
        assert_ne!(
            hash_small, hash_standard,
            "different map sizes produced identical gradient PNGs"
        );
    }
}

// ------------- Reference hash (ignored until Stage 3.1) -------------

mod reference_hash {
    use super::*;

    /// blake3 hash of the pixel PNG of a Tectonics-sub-RNG-filled field
    /// for seed `0xDEAD_BEEF` on a `Small` map (120×80 tiles).
    ///
    /// **This value must be updated whenever generation output changes.**
    /// Run with `--ignored --nocapture` to print the current hash, then
    /// paste it here and commit alongside the generation change.
    ///
    /// Set to all-zeros as a placeholder until Stage 3.1 produces real
    /// generation data worth committing.
    const REFERENCE_SEEDED_FIELD_HASH: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    #[ignore = "placeholder: un-ignore in Stage 3.1 once tectonics produces real fields"]
    fn seeded_field_matches_committed_reference_hash() {
        let params = test_params(0xDEAD_BEEF);
        let hash = seeded_field_hash_for(&params);
        let hash_hex = hash.to_hex();

        // Print the current hash so it's easy to update.
        println!("current seeded field hash: {hash_hex}");
        println!("expected:                  {REFERENCE_SEEDED_FIELD_HASH}");

        assert_eq!(
            hash_hex.as_str(),
            REFERENCE_SEEDED_FIELD_HASH,
            "seeded field hash changed — if this is intentional, update \
             REFERENCE_SEEDED_FIELD_HASH and commit alongside the generation change"
        );
    }

    /// blake3 hash of the `WorldData` (currently just `MapBounds`) for
    /// seed `0xDEAD_BEEF` on a `Small` map.
    ///
    /// This test is low-value while `WorldData` is a stub. It becomes
    /// meaningful in Stage 3.1 when elevation and plate fields are added.
    /// Un-ignore then, update the hash, and extend `world_data_hash` to
    /// cover the new fields.
    const REFERENCE_WORLD_DATA_HASH: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    #[ignore = "placeholder: un-ignore in Stage 3.1 once WorldData carries real fields"]
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
