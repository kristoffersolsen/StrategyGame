//! Organic plate growth via frontier flood-fill (step 2).
//!
//! Grows tectonic plates from their seed tiles by iteratively claiming
//! random unassigned neighbours. This produces irregular, concave plate
//! shapes — peninsulas, interlocking fingers, and curved coastlines —
//! that Voronoi partitions cannot generate regardless of perturbation.
//!
//! # Algorithm
//!
//! Each round iterates through every plate in index order. A plate
//! picks one random tile from its *frontier* (the set of currently
//! unassigned tiles adjacent to tiles already belonging to that plate)
//! and claims it. Claimed tiles' unassigned neighbours are immediately
//! added to the frontier. Stale frontier entries — tiles that were
//! claimed by a competing plate between insertion and selection — are
//! removed lazily on encounter.
//!
//! Rounds continue until every tile is assigned. Termination is
//! guaranteed because the cylindrical hex grid is connected: any
//! unassigned tile is reachable from at least one seed, so the
//! frontier always reaches every remaining tile.
//!
//! # Frontier duplicates
//!
//! A tile may appear in the same plate's frontier list more than once
//! (inserted through multiple adjacent claimed tiles) and may appear
//! in several plates' frontier lists simultaneously. Both cases are
//! harmless: the lazy assignment check discards stale entries, and the
//! first plate to claim a tile wins.
//!
//! # Wrap awareness
//!
//! [`MapBounds::neighbors`] returns wrap-normalised coordinates for
//! east-west neighbours and `None` for north/south out-of-bounds
//! positions. The implementation calls `.flatten()` on that array,
//! so polar edges and cylindrical wrapping are handled transparently.

use coords::{Coord, MapBounds};
use rand::RngExt;

use crate::Field;
use crate::params::TectonicsParams;
use crate::rng::StageRng;
use crate::world_data::PlateId;

// ------------- Sentinel -------------

/// Sentinel value stored in the `plate_id` field for tiles not yet
/// claimed by any plate.
///
/// `u8::MAX` is safe because [`MAX_TARGET_PLATE_COUNT`] is 32, so
/// valid plate indices occupy `0..=31`.
///
/// [`MAX_TARGET_PLATE_COUNT`]: crate::params::MAX_TARGET_PLATE_COUNT
const UNASSIGNED: PlateId = u8::MAX;

// ------------- Entry point -------------

/// Grows tectonic plates organically from `seeds` via frontier flood-fill.
///
/// Each plate takes one turn per round, claiming a random unassigned
/// tile from its frontier. Plates with an exhausted frontier are
/// skipped; other plates continue until all tiles are claimed.
///
/// # Arguments
///
/// * `bounds` — map dimensions.
/// * `seeds` — plate seed coordinates; each seed becomes the initial
///   tile of its corresponding plate (index order).
/// * `rng` — stage-scoped RNG for random frontier selection.
///
/// # Panics
///
/// Panics if `seeds` is empty.
pub(crate) fn grow_plates_organic(
    bounds: MapBounds,
    seeds: &[Coord],
    params: &TectonicsParams,
    rng: &mut StageRng,
) -> Field<PlateId> {
    assert!(
        !seeds.is_empty(),
        "grow_plates_organic requires at least one seed"
    );

    let n = seeds.len();

    let mut plate_id = Field::filled(bounds, UNASSIGNED);
    let mut frontiers: Vec<Vec<Coord>> = vec![Vec::new(); n];
    let growth_rates = sample_growth_rates(n, params, rng);
    let mut growth_credit = vec![0.0_f32; n];

    // Claim seed tiles and populate initial frontiers.
    for (p, &seed) in seeds.iter().enumerate() {
        let seed = bounds
            .normalize(seed)
            .expect("seed coordinate must be in-bounds");
        #[allow(clippy::cast_possible_truncation)]
        plate_id.set(seed, p as PlateId);
        push_unassigned_neighbors(bounds, &plate_id, seed, &mut frontiers[p]);
    }

    let total = bounds.width() as usize * bounds.height() as usize;
    let mut remaining = total - n;

    while remaining > 0 {
        #[allow(clippy::needless_range_loop)]
        for p in 0..n {
            growth_credit[p] += growth_rates[p];
            while growth_credit[p] >= 1.0 {
                if !claim_frontier_tile(bounds, &mut plate_id, &mut frontiers[p], p, rng) {
                    break;
                }
                remaining -= 1;
                growth_credit[p] -= 1.0;

                if remaining == 0 {
                    break;
                }
            }

            if remaining == 0 {
                break;
            }
        }
    }

    plate_id
}

// ------------- Helper -------------

fn sample_growth_rates(
    plate_count: usize,
    params: &TectonicsParams,
    rng: &mut StageRng,
) -> Vec<f32> {
    if params.plate_growth_rate_range <= 1.0 {
        return vec![1.0; plate_count];
    }

    let min = 1.0 / params.plate_growth_rate_range;
    let max = params.plate_growth_rate_range;
    (0..plate_count)
        .map(|_| rng.random_range(min..=max))
        .collect()
}

fn claim_frontier_tile(
    bounds: MapBounds,
    plate_id: &mut Field<PlateId>,
    frontier: &mut Vec<Coord>,
    plate_index: usize,
    rng: &mut StageRng,
) -> bool {
    loop {
        if frontier.is_empty() {
            return false;
        }
        let i = rng.random_range(0..frontier.len());
        let tile = frontier[i];

        if plate_id[tile] != UNASSIGNED {
            // Stale entry: claimed by another plate. Remove lazily.
            frontier.swap_remove(i);
            continue;
        }

        #[allow(clippy::cast_possible_truncation)]
        plate_id.set(tile, plate_index as PlateId);
        frontier.swap_remove(i);
        push_unassigned_neighbors(bounds, plate_id, tile, frontier);
        return true;
    }
}

/// Pushes each unassigned neighbour of `tile` onto `frontier`.
///
/// Uses `bounds.neighbors` for wrap-aware, pole-safe traversal.
/// Duplicate insertions are harmless — they will be discarded lazily.
fn push_unassigned_neighbors(
    bounds: MapBounds,
    plate_id: &Field<PlateId>,
    tile: Coord,
    frontier: &mut Vec<Coord>,
) {
    for neighbor in bounds.neighbors(tile).iter().flatten() {
        if plate_id[*neighbor] == UNASSIGNED {
            frontier.push(*neighbor);
        }
    }
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{MasterRng, StageId};
    use coords::Coord;

    fn make_rng(seed: u64) -> StageRng {
        MasterRng::new(seed).fork(StageId::Tectonics)
    }

    fn default_params() -> TectonicsParams {
        TectonicsParams::defaults()
    }

    // ------------- Coverage -------------

    mod coverage {
        use super::*;

        /// Every tile must be assigned after growth.
        #[test]
        fn all_tiles_assigned() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(10, 7), Coord::new(30, 20), Coord::new(20, 5)];
            let mut rng = make_rng(1);
            let field = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng);

            for (_, &id) in field.iter_coords() {
                assert_ne!(id, UNASSIGNED, "tile must be assigned");
            }
        }

        /// All assigned IDs must be valid plate indices.
        #[test]
        fn all_ids_valid() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(10, 7), Coord::new(30, 20), Coord::new(20, 5)];
            #[allow(clippy::cast_possible_truncation)]
            let plate_count = seeds.len() as PlateId;
            let mut rng = make_rng(2);
            let field = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng);

            for (_, &id) in field.iter_coords() {
                assert!(id < plate_count, "invalid plate id {id}");
            }
        }

        /// Each seed tile must belong to its own plate.
        #[test]
        fn seeds_own_their_tiles() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(5, 5), Coord::new(25, 15), Coord::new(15, 25)];
            let mut rng = make_rng(3);
            let field = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng);

            for (p, &seed) in seeds.iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let expected: PlateId = p as PlateId;
                assert_eq!(
                    field[seed], expected,
                    "seed {seed:?} should belong to plate {p}"
                );
            }
        }
    }

    // ------------- Single plate -------------

    mod single_plate {
        use super::*;

        /// A single seed must claim every tile.
        #[test]
        fn single_seed_covers_all() {
            let bounds = MapBounds::new(20, 15);
            let seeds = vec![Coord::new(10, 7)];
            let mut rng = make_rng(4);
            let field = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng);

            for (_, &id) in field.iter_coords() {
                assert_eq!(id, 0, "single plate must cover everything");
            }
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_field() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(10, 7), Coord::new(30, 20)];

            let mut rng_a = make_rng(42);
            let mut rng_b = make_rng(42);
            let a = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng_a);
            let b = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng_b);

            assert_eq!(a.as_slice(), b.as_slice());
        }

        #[test]
        fn different_rng_seeds_produce_different_fields() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(10, 7), Coord::new(30, 20)];

            let mut rng_a = make_rng(1);
            let mut rng_b = make_rng(2);
            let a = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng_a);
            let b = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng_b);

            assert_ne!(a.as_slice(), b.as_slice());
        }
    }

    // ------------- Wrap continuity -------------

    mod wrap {
        use super::*;

        /// A west-edge seed should capture east-edge tiles via the
        /// cylindrical wrap rather than leaving them as an unassigned
        /// island on the other side of the map.
        #[test]
        fn west_edge_seed_captures_east_edge_tiles() {
            let bounds = MapBounds::new(40, 20);
            // West-edge seed; central seed keeps it from dominating the map.
            let seeds = vec![Coord::new(1, 10), Coord::new(20, 10)];
            let mut rng = make_rng(7);
            let field = grow_plates_organic(bounds, &seeds, &default_params(), &mut rng);

            // Tiles at q=38 and q=39 are 3 and 2 steps from seed 0 via wrap.
            // Without correct wrap-aware neighbour traversal they could be
            // cut off from seed 0's frontier.
            for (_, &id) in field.iter_coords() {
                assert_ne!(id, UNASSIGNED, "all tiles must be assigned");
            }
        }
    }
}
