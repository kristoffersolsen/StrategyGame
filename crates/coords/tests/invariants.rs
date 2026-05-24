//! Property-based invariant tests for the `coords` crate.
//!
//! These tests verify mathematical properties that must hold for all
//! possible inputs, not just the specific cases covered by unit tests.
//! `proptest` generates hundreds of random inputs per property and
//! shrinks failures to minimal reproducing cases automatically.

use coords::{Coord, MapBounds};
use proptest::prelude::*;

// ------------- Strategies -------------

// A "strategy" in proptest is a description of how to generate a value.
// These are reusable building blocks for the tests below.

/// Generates a valid `MapBounds` with width in `[2, 100]` and height in `[1, 100]`.
fn any_bounds() -> impl Strategy<Value = MapBounds> {
    (2u32..=100, 1u32..=100).prop_map(|(w, h)| MapBounds::new(w, h))
}

/// Generates a `Coord` that is guaranteed in-bounds for the given `MapBounds`.
fn any_coord_in(bounds: MapBounds) -> impl Strategy<Value = Coord> {
    let w = bounds.width();
    let h = bounds.height();
    (0..w, 0..h).prop_map(move |(q, r)| Coord::new(q.cast_signed(), r.cast_signed()))
}

// ------------- Invariants -------------

proptest! {
    /// Wrapping a coordinate twice produces the same result as wrapping once.
    #[test]
    fn normalize_is_idempotent(
        bounds in any_bounds(),
        coord in any_bounds().prop_flat_map(any_coord_in)
    ) {
        // Generate a coord in-bounds for *some* map, then test it against
        // our bounds — it may be out of bounds, which is fine: normalize
        // handles that by returning None.
        let first = bounds.normalize(coord);
        let second = first.and_then(|c| bounds.normalize(c));
        prop_assert_eq!(first, second);
    }

    /// Distance is symmetric: d(a, b) == d(b, a).
    #[test]
    fn distance_is_symmetric(
        bounds in any_bounds(),
        a in any_bounds().prop_flat_map(any_coord_in),
        b in any_bounds().prop_flat_map(any_coord_in),
    ) {
        // Normalize both coords into our bounds before measuring.
        // Out-of-bounds coords are skipped via prop_assume.
        let Some(a) = bounds.normalize(a) else { return Ok(()) };
        let Some(b) = bounds.normalize(b) else { return Ok(()) };
        prop_assert_eq!(bounds.distance(a, b), bounds.distance(b, a));
    }

    /// Every coord returned by `range` is within the requested step count.
    #[test]
    fn range_results_are_within_distance(
        bounds in any_bounds(),
        steps in 0i32..=5,
        center in any_bounds().prop_flat_map(any_coord_in),
    ) {
        let Some(center) = bounds.normalize(center) else { return Ok(()) };
        for tile in bounds.range(center, steps) {
            prop_assert!(
                bounds.distance(center, tile) <= steps,
                "tile {:?} is {} steps from {:?}, expected <= {}",
                tile, bounds.distance(center, tile), center, steps
            );
        }
    }

    /// Every in-bounds neighbor is exactly distance 1 from its origin.
    #[test]
    fn neighbors_are_distance_one(
        bounds in any_bounds(),
        coord in any_bounds().prop_flat_map(any_coord_in),
    ) {
        let Some(coord) = bounds.normalize(coord) else { return Ok(()) };
        for neighbor in bounds.neighbors(coord).into_iter().flatten() {
            prop_assert_eq!(
                bounds.distance(coord, neighbor),
                1,
                "neighbor {:?} of {:?} should be distance 1",
                neighbor, coord
            );
        }
    }
}
