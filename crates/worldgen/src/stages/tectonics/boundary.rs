//! Boundary classification between tectonic plates (step 5).
//!
//! Identifies every tile adjacent to a tile of a different plate
//! (boundary tiles) and classifies the interaction between the two
//! plates as convergent, divergent, or transform.
//!
//! # Algorithm
//!
//! 1. For each ordered pair `(a, b)` where `a < b`, compute the
//!    wrap-aware unit normal from seed `a` to seed `b`.
//! 2. Project the relative motion `motion_a − motion_b` onto the
//!    normal to obtain an *approach velocity*.  Positive means the
//!    plates are converging; negative means they are diverging.
//! 3. Compare the approach velocity against `convergence_threshold`:
//!    above → [`BoundaryType::Convergent`],
//!    below the negative threshold → [`BoundaryType::Divergent`],
//!    otherwise → [`BoundaryType::Transform`].
//! 4. For each tile, check all six hex-neighbors. Tiles adjacent to
//!    at least one tile of a different plate are boundary tiles; the
//!    stored type is the most significant boundary among all adjacent
//!    plate pairs (convergent > divergent > transform).

use std::collections::BTreeMap;

use coords::{Coord, MapBounds};

use crate::Field;
use crate::world_data::{BoundaryType, MotionVector, Plate, PlateId};

// ------------- Public entry point -------------

/// Classifies boundary tiles and returns a per-tile boundary-type field.
///
/// Interior tiles (not adjacent to any tile of a different plate) are
/// `None`; boundary tiles carry `Some(`[`BoundaryType`]`)`. When a tile
/// borders more than one other plate the most geologically significant
/// type wins (convergent > divergent > transform).
///
/// # Panics
///
/// Panics if `plates.len() != seeds.len()`, which is the invariant
/// established by step 4.
pub(crate) fn classify_boundaries(
    bounds: MapBounds,
    plate_id: &Field<PlateId>,
    plates: &[Plate],
    seeds: &[Coord],
    convergence_threshold: f32,
) -> Field<Option<BoundaryType>> {
    assert_eq!(
        plates.len(),
        seeds.len(),
        "plates and seeds must be parallel (invariant from step 4)"
    );

    // Pre-compute boundary type for every plate pair using a BTreeMap so
    // iteration order is deterministic if we ever need to iterate it.
    let pair_types = classify_all_pairs(plates, seeds, bounds, convergence_threshold);

    Field::from_fn(bounds, |tile| {
        let my_id = plate_id[tile];
        let mut best: Option<BoundaryType> = None;

        for neighbor in bounds.neighbors(tile).iter().flatten() {
            let adj_id = plate_id[*neighbor];
            if adj_id != my_id {
                let pair_type = pair_types[&plate_pair_key(my_id, adj_id)];
                best = Some(match best {
                    None => pair_type,
                    Some(current) => more_significant(current, pair_type),
                });
            }
        }
        best
    })
}

// ------------- Pair-level classification -------------

/// Classifies every ordered plate pair and returns a lookup map from
/// canonical pair keys to [`BoundaryType`].
///
/// The canonical key is `(min_id, max_id)` so each pair appears once.
fn classify_all_pairs(
    plates: &[Plate],
    seeds: &[Coord],
    bounds: MapBounds,
    threshold: f32,
) -> BTreeMap<(PlateId, PlateId), BoundaryType> {
    let mut map = BTreeMap::new();

    for a in 0..plates.len() {
        for b in (a + 1)..plates.len() {
            #[allow(clippy::cast_possible_truncation)]
            let id_a = a as PlateId;
            #[allow(clippy::cast_possible_truncation)]
            let id_b = b as PlateId;
            let bt = classify_pair(
                seeds[a],
                seeds[b],
                plates[a].motion,
                plates[b].motion,
                bounds,
                threshold,
            );
            map.insert((id_a, id_b), bt);
        }
    }

    map
}

/// Returns the canonical (min, max) key for a plate pair.
fn plate_pair_key(a: PlateId, b: PlateId) -> (PlateId, PlateId) {
    if a < b { (a, b) } else { (b, a) }
}

/// Classifies the interaction between two plates based on their motion
/// vectors and the wrap-aware boundary normal between their seeds.
fn classify_pair(
    seed_a: Coord,
    seed_b: Coord,
    motion_a: MotionVector,
    motion_b: MotionVector,
    bounds: MapBounds,
    threshold: f32,
) -> BoundaryType {
    let (nx, ny) = boundary_normal(seed_a, seed_b, bounds);

    // Approach velocity: positive means a is moving toward b faster
    // than b is moving away, i.e. the plates are converging.
    let approach = (motion_a.dx - motion_b.dx) * nx + (motion_a.dy - motion_b.dy) * ny;

    if approach > threshold {
        BoundaryType::Convergent
    } else if approach < -threshold {
        BoundaryType::Divergent
    } else {
        BoundaryType::Transform
    }
}

/// Returns the wrap-aware unit normal vector from `seed_a` to `seed_b`.
///
/// Takes the shorter east-west path for the q component. Returns an
/// arbitrary unit vector if the seeds are at the same position (which
/// should not happen for a valid plate set).
// q and r are naturally "similar" names in hex coordinate space; float
// intermediates must follow the same convention.
#[allow(clippy::similar_names)]
fn boundary_normal(seed_a: Coord, seed_b: Coord, bounds: MapBounds) -> (f32, f32) {
    let dq_raw = seed_b.q - seed_a.q;
    let dr = seed_b.r - seed_a.r;

    let width_i32 = bounds.width().cast_signed();
    let half_width = width_i32 / 2;

    // Choose the shorter arc: if the direct distance wraps, adjust.
    let dq = if dq_raw > half_width {
        dq_raw - width_i32
    } else if dq_raw < -half_width {
        dq_raw + width_i32
    } else {
        dq_raw
    };

    #[allow(clippy::cast_precision_loss)]
    let (dq_f, dr_f) = (dq as f32, dr as f32);
    let len = (dq_f * dq_f + dr_f * dr_f).sqrt();

    if len < f32::EPSILON {
        // Seeds at the same position: degenerate, return arbitrary unit vector.
        return (1.0, 0.0);
    }

    (dq_f / len, dr_f / len)
}

// ------------- Severity ordering -------------

/// Returns the more geologically significant of two boundary types.
///
/// Convergent > Divergent > Transform. Used when a tile borders more
/// than one plate and we must pick a single representative type.
fn more_significant(a: BoundaryType, b: BoundaryType) -> BoundaryType {
    if a == BoundaryType::Convergent || b == BoundaryType::Convergent {
        BoundaryType::Convergent
    } else if a == BoundaryType::Divergent || b == BoundaryType::Divergent {
        BoundaryType::Divergent
    } else {
        BoundaryType::Transform
    }
}

// ------------- Tests -------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_data::{MineralCharacter, PlateType};

    fn make_plate(dx: f32, dy: f32, plate_type: PlateType) -> Plate {
        use crate::world_data::MotionVector;
        Plate {
            plate_type,
            motion: MotionVector { dx, dy },
            mineral_character: MineralCharacter::Ferrous,
        }
    }

    fn test_bounds() -> MapBounds {
        MapBounds::new(40, 30)
    }

    // ------------- Pair classification -------------

    mod pair_classification {
        use super::*;

        /// Seeds at q=10 and q=25 on a width-40 map: direct distance 15,
        /// wrap distance 25.  The shorter eastward path gives a boundary
        /// normal of (+1, 0).
        fn east_west_seeds() -> (MapBounds, Coord, Coord) {
            let bounds = test_bounds(); // 40 wide
            (bounds, Coord::new(10, 15), Coord::new(25, 15))
        }

        #[test]
        fn head_on_approach_is_convergent() {
            let (bounds, seed_a, seed_b) = east_west_seeds();
            // Normal points east (+1, 0). A moves east, B moves west: converging.
            let motion_a = MotionVector { dx: 1.0, dy: 0.0 };
            let motion_b = MotionVector { dx: -1.0, dy: 0.0 };
            let bt = classify_pair(seed_a, seed_b, motion_a, motion_b, bounds, 0.3);
            assert_eq!(bt, BoundaryType::Convergent);
        }

        #[test]
        fn head_on_separation_is_divergent() {
            let (bounds, seed_a, seed_b) = east_west_seeds();
            // A moves west, B moves east: diverging.
            let motion_a = MotionVector { dx: -1.0, dy: 0.0 };
            let motion_b = MotionVector { dx: 1.0, dy: 0.0 };
            let bt = classify_pair(seed_a, seed_b, motion_a, motion_b, bounds, 0.3);
            assert_eq!(bt, BoundaryType::Divergent);
        }

        #[test]
        fn perpendicular_motion_is_transform() {
            let (bounds, seed_a, seed_b) = east_west_seeds();
            // Both moving southward: tangential to the east-west boundary.
            let motion_a = MotionVector { dx: 0.0, dy: 1.0 };
            let motion_b = MotionVector { dx: 0.0, dy: 1.0 };
            let bt = classify_pair(seed_a, seed_b, motion_a, motion_b, bounds, 0.3);
            assert_eq!(bt, BoundaryType::Transform);
        }

        #[test]
        fn zero_threshold_classifies_every_nontangential_boundary() {
            // With threshold=0, any nonzero approach velocity classifies
            // as convergent or divergent, never transform.
            let (bounds, seed_a, seed_b) = east_west_seeds();
            let motion_a = MotionVector { dx: 0.1, dy: 0.0 };
            let motion_b = MotionVector { dx: -0.1, dy: 0.0 };
            let bt = classify_pair(seed_a, seed_b, motion_a, motion_b, bounds, 0.0);
            assert_eq!(bt, BoundaryType::Convergent);
        }
    }

    // ------------- Per-tile field assignment -------------

    mod field_assignment {
        use super::*;
        use crate::params::TectonicsParams;
        use crate::rng::{MasterRng, StageId};
        use crate::stages::tectonics::growth::grow_plates_organic;

        fn two_plate_setup() -> (MapBounds, Vec<Coord>, Vec<Plate>) {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(10, 15), Coord::new(30, 15)];
            // A moves right, B moves left: convergent boundary between them.
            let plates = vec![
                make_plate(1.0, 0.0, PlateType::Continental),
                make_plate(-1.0, 0.0, PlateType::Oceanic),
            ];
            (bounds, seeds, plates)
        }

        #[test]
        fn interior_tiles_are_none() {
            let (bounds, seeds, plates) = two_plate_setup();
            let mut rng = MasterRng::new(42).fork(StageId::Tectonics);
            let plate_id =
                grow_plates_organic(bounds, &seeds, &TectonicsParams::defaults(), &mut rng);
            let bt = classify_boundaries(bounds, &plate_id, &plates, &seeds, 0.3);

            // The seed at (10, 15) is deep inside plate 0 — should be interior.
            let interior = Coord::new(10, 15);
            assert_eq!(
                *bt.get(interior).unwrap(),
                None,
                "seed tile should be interior"
            );
        }

        #[test]
        fn boundary_tiles_are_some() {
            let (bounds, seeds, plates) = two_plate_setup();
            let mut rng = MasterRng::new(42).fork(StageId::Tectonics);
            let plate_id =
                grow_plates_organic(bounds, &seeds, &TectonicsParams::defaults(), &mut rng);
            let bt = classify_boundaries(bounds, &plate_id, &plates, &seeds, 0.3);

            // At least one tile on the map must be a boundary tile.
            let has_boundary = bt.iter().any(Option::is_some);
            assert!(has_boundary, "no boundary tiles found");
        }

        #[test]
        fn converging_plates_produce_convergent_boundary() {
            let (bounds, seeds, plates) = two_plate_setup();
            let mut rng = MasterRng::new(42).fork(StageId::Tectonics);
            let plate_id =
                grow_plates_organic(bounds, &seeds, &TectonicsParams::defaults(), &mut rng);
            let bt = classify_boundaries(bounds, &plate_id, &plates, &seeds, 0.3);

            // The boundary between the two seeds should be convergent
            // since A moves right and B moves left.
            let boundary_tiles: Vec<_> = bt.iter_coords().filter(|(_, v)| v.is_some()).collect();
            assert!(
                !boundary_tiles.is_empty(),
                "expected boundary tiles between convergent plates"
            );
            for (_, v) in &boundary_tiles {
                assert_eq!(
                    **v,
                    Some(BoundaryType::Convergent),
                    "boundary tile should be convergent"
                );
            }
        }
    }

    // ------------- Severity ordering -------------

    mod severity {
        use super::*;

        #[test]
        fn convergent_beats_divergent() {
            assert_eq!(
                more_significant(BoundaryType::Convergent, BoundaryType::Divergent),
                BoundaryType::Convergent
            );
            assert_eq!(
                more_significant(BoundaryType::Divergent, BoundaryType::Convergent),
                BoundaryType::Convergent
            );
        }

        #[test]
        fn divergent_beats_transform() {
            assert_eq!(
                more_significant(BoundaryType::Divergent, BoundaryType::Transform),
                BoundaryType::Divergent
            );
        }

        #[test]
        fn same_type_returns_same_type() {
            for bt in [
                BoundaryType::Convergent,
                BoundaryType::Divergent,
                BoundaryType::Transform,
            ] {
                assert_eq!(more_significant(bt, bt), bt);
            }
        }
    }

    // ------------- Wrap-awareness -------------

    mod wrap {
        use super::*;

        /// Seed A is near the west edge (q=1), seed B near the east edge
        /// (q=38) on a width-40 map. The wrap-aware boundary normal should
        /// point westward (from B toward A via the seam), not across the
        /// full width of the map.
        #[test]
        fn boundary_normal_uses_shorter_wrap_path() {
            let bounds = MapBounds::new(40, 30);
            // Seed A near west edge, B near east edge.
            // Wrap distance: 40 - 38 + 1 = 3.  Direct distance: 38 - 1 = 37.
            // Wrap path is shorter; dq should be -3 (B is 3 steps "west" of A via wrap).
            let (nx, _ny) = boundary_normal(Coord::new(1, 15), Coord::new(38, 15), bounds);
            // The normal from A to B via the wrap is eastward from A's view
            // (A is at q=1, B is at q=38 which wraps to q=-2 relative to A).
            // dq = 38 - 1 = 37; but 37 > 20 so adjusted to 37 - 40 = -3.
            // So the normal points westward: nx < 0.
            assert!(
                nx < 0.0,
                "normal should use the short wrap path (nx = {nx})"
            );
        }
    }

    // ------------- Determinism -------------

    mod determinism {
        use super::*;
        use crate::params::TectonicsParams;
        use crate::rng::{MasterRng, StageId};
        use crate::stages::tectonics::growth::grow_plates_organic;

        #[test]
        fn same_inputs_produce_identical_output() {
            let bounds = MapBounds::new(40, 30);
            let seeds = vec![Coord::new(10, 7), Coord::new(30, 20), Coord::new(20, 5)];
            let plates = vec![
                make_plate(1.0, 0.0, PlateType::Continental),
                make_plate(-1.0, 0.5, PlateType::Oceanic),
                make_plate(0.0, -1.0, PlateType::Continental),
            ];
            let mut rng = MasterRng::new(42).fork(StageId::Tectonics);
            let plate_id =
                grow_plates_organic(bounds, &seeds, &TectonicsParams::defaults(), &mut rng);

            let a = classify_boundaries(bounds, &plate_id, &plates, &seeds, 0.3);
            let b = classify_boundaries(bounds, &plate_id, &plates, &seeds, 0.3);

            assert_eq!(a.as_slice(), b.as_slice());
        }
    }
}
