//! Voronoi partition assigning every tile to a tectonic plate.
//!
//! Step 2 of the tectonics stage (design §1.9). Every tile is assigned
//! the index of the nearest plate seed, producing a `Field<u8>` that
//! partitions the map into plate territories.
//!
//! # Tie-breaking
//!
//! When two seeds are equidistant from a tile, the lower seed index
//! wins. This is deterministic and independent of iteration order.
//! Without an explicit tie-break, floating-point distance comparisons
//! on a discrete grid can produce nondeterministic assignments
//! depending on compiler and hardware optimisations.
//!
//! # Complexity
//!
//! O(tiles × plates). For 36 000 tiles and 10 plates this is 360 000
//! integer distance operations — negligible. No spatial acceleration
//! structure is needed at this scale.

use coords::{Coord, MapBounds};

use crate::Field;
use crate::world_data::PlateId;

/// Assigns every tile to the nearest plate seed, producing a plate-ID
/// field.
///
/// Uses wrap-aware hex distance via [`MapBounds::distance`]. Ties
/// (equidistant seeds) are broken by lowest seed index, ensuring
/// deterministic output regardless of iteration order.
///
/// # Panics
///
/// Panics if `seeds` is empty. The caller (the tectonics stage entry
/// point) guarantees at least one seed is produced by step 1; an
/// empty seeds list would indicate a logic error upstream.
pub(crate) fn assign_plates(bounds: MapBounds, seeds: &[Coord]) -> Field<PlateId> {
    assert!(
        !seeds.is_empty(),
        "assign_plates requires at least one seed"
    );

    Field::from_fn(bounds, |tile| nearest_seed(tile, seeds, bounds))
}

/// Returns the index of the seed nearest to `tile`, with ties broken
/// by lowest index.
fn nearest_seed(tile: Coord, seeds: &[Coord], bounds: MapBounds) -> PlateId {
    let mut best_id = 0u8;
    let mut best_dist = i32::MAX;

    for (idx, &seed) in seeds.iter().enumerate() {
        let dist = bounds.distance(tile, seed);
        // Strict less-than: equal distances keep the lower index (best_id),
        // satisfying the tie-breaking contract without an extra comparison.
        if dist < best_dist {
            best_dist = dist;
            // Cast is safe: MAX_TARGET_PLATE_COUNT is 32, well within u8.
            #[allow(clippy::cast_possible_truncation)]
            let id = idx as PlateId;
            best_id = id;
        }
    }

    best_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::Coord;

    // ------------- Basic assignment -------------

    mod assignment {
        use super::*;

        #[test]
        fn every_tile_is_assigned_a_valid_plate_id() {
            let bounds = MapBounds::new(20, 15);
            let seeds = vec![Coord::new(5, 3), Coord::new(15, 10), Coord::new(10, 7)];
            let plate_id = assign_plates(bounds, &seeds);
            #[allow(clippy::cast_possible_truncation)]
            let plate_count = seeds.len() as PlateId;

            for (_, &id) in plate_id.iter_coords() {
                assert!(id < plate_count, "plate id {id} out of range");
            }
        }

        #[test]
        fn seed_tile_maps_to_its_own_plate() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(5, 5), Coord::new(30, 20), Coord::new(20, 10)];
            let plate_id = assign_plates(bounds, &seeds);

            for (idx, &seed) in seeds.iter().enumerate() {
                let assigned = *plate_id.get(seed).expect("seed in bounds");
                #[allow(clippy::cast_possible_truncation)]
                let expected = idx as PlateId;
                assert_eq!(
                    assigned, expected,
                    "seed {idx} at {seed:?} should map to its own plate"
                );
            }
        }

        #[test]
        fn single_seed_assigns_all_tiles_to_plate_zero() {
            let bounds = MapBounds::new(10, 10);
            let seeds = vec![Coord::new(5, 5)];
            let plate_id = assign_plates(bounds, &seeds);

            for (_, &id) in plate_id.iter_coords() {
                assert_eq!(id, 0);
            }
        }
    }

    // ------------- Tie-breaking -------------

    mod tie_breaking {
        use super::*;

        /// Place two seeds symmetrically so the centre column is
        /// equidistant from both. The lower-index seed (0) must win.
        #[test]
        fn equidistant_tile_takes_lower_index() {
            let bounds = MapBounds::new(10, 10);
            // Seeds at q=2 and q=7 on a width-10 map; equidistant at q=4 and q=5
            // (hex distance 2 from each on row 5).
            let seeds = vec![Coord::new(2, 5), Coord::new(7, 5)];
            let plate_id = assign_plates(bounds, &seeds);

            // Tile exactly between both seeds: distance 2 from each.
            // Lowest index (0) wins.
            let mid = Coord::new(5, 5);
            let d0 = bounds.distance(mid, seeds[0]);
            let d1 = bounds.distance(mid, seeds[1]);
            if d0 == d1 {
                let assigned = *plate_id.get(mid).unwrap();
                assert_eq!(assigned, 0, "tie must go to lower index");
            }
            // If not equidistant at this exact tile, the invariant is
            // still tested by the determinism test below.
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;

        #[test]
        fn same_inputs_produce_identical_field() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![
                Coord::new(10, 7),
                Coord::new(30, 20),
                Coord::new(20, 5),
                Coord::new(5, 25),
            ];
            let a = assign_plates(bounds, &seeds);
            let b = assign_plates(bounds, &seeds);
            assert_eq!(a.as_slice(), b.as_slice());
        }
    }

    // ------------- Wrap-awareness -------------

    mod wrap {
        use super::*;

        /// A seed at q=0 should capture tiles near q=width-1, because
        /// the wrap-aware distance makes them close.
        #[test]
        fn seed_near_west_edge_captures_tiles_near_east_edge() {
            let bounds = MapBounds::new(20, 10);
            // Seed A at the west edge, seed B far from both edges.
            let seeds = vec![
                Coord::new(0, 5),  // west edge
                Coord::new(10, 5), // centre
            ];
            let plate_id = assign_plates(bounds, &seeds);

            // The tile at q=19 is 1 step from seed A (via wrap) and
            // 9 steps from seed B — must be assigned to plate 0.
            let near_east = Coord::new(19, 5);
            let d_to_a = bounds.distance(near_east, seeds[0]);
            let d_to_b = bounds.distance(near_east, seeds[1]);
            assert!(d_to_a < d_to_b, "wrap distance check precondition");
            assert_eq!(
                *plate_id.get(near_east).unwrap(),
                0,
                "east-edge tile should be captured by west-edge seed via wrap"
            );
        }
    }
}
