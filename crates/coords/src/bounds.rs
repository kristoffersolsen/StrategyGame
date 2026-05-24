//! Map bounds and wrap-aware coordinate operations.

use std::fmt;

use crate::{Coord, Direction};

/// The dimensions of a map, defining what wraps and what is out of bounds.
///
/// The width axis (`q`) wraps toroidally; the height axis (`r`) is bounded.
/// A `MapBounds` is passed explicitly to every wrap- or bounds-aware
/// coordinate operation, so a [`Coord`] never needs to carry a back-
/// reference to its map.
///
/// Width and height are stored as `u32` because they are non-negative by
/// construction. They are converted to `i32` at the few sites that need
/// signed arithmetic with [`Coord`] fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapBounds {
    width: u32,
    height: u32,
}

impl MapBounds {
    /// Constructs a [`MapBounds`] with the given width and height.
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` is `0`. A zero-dimension map has no
    /// valid coordinates, so any such bounds value would make every
    /// operation ill-defined. This is a programmer error, not a runtime
    /// condition, so it panics rather than returning [`Option`].
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        assert!(width > 0, "map width must be non-zero");
        assert!(height > 0, "map height must be non-zero");
        assert!(width <= i32::MAX as u32, "map width must fit in i32");
        assert!(height <= i32::MAX as u32, "map height must fit in i32");
        Self { width, height }
    }

    /// Returns the wrap-axis extent.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Returns the bounded-axis extent.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Returns `true` if `coord` is within bounds after wrap normalization.
    ///
    /// The `q` axis always wraps, so only `r` can be out of bounds.
    #[must_use]
    pub const fn contains(self, coord: Coord) -> bool {
        coord.r >= 0 && coord.r < self.height.cast_signed()
    }

    /// Returns the canonical in-bounds form of `coord`.
    ///
    /// Wraps `q` toroidally into `[0, width)`. Returns `None` if `r` is
    /// outside `[0, height)` — north-south is bounded, not wrapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use coords::{Coord, MapBounds};
    /// let bounds = MapBounds::new(10, 10);
    ///
    /// // q wraps around
    /// assert_eq!(bounds.normalize(Coord::new(10, 0)), Some(Coord::new(0, 0)));
    /// assert_eq!(bounds.normalize(Coord::new(-1, 0)), Some(Coord::new(9, 0)));
    ///
    /// // r out of bounds returns None
    /// assert_eq!(bounds.normalize(Coord::new(0, -1)), None);
    /// assert_eq!(bounds.normalize(Coord::new(0, 10)), None);
    /// ```
    #[must_use]
    pub fn normalize(self, coord: Coord) -> Option<Coord> {
        if !self.contains(coord) {
            return None;
        }
        let w = self.width.cast_signed();
        let q = ((coord.q % w) + w) % w;
        Some(Coord::new(q, coord.r))
    }

    /// Returns the neighbor of `coord` in `direction`, or `None` if the
    /// neighbor would fall off the north or south edge.
    ///
    /// East-west neighbors always exist and are wrapped into bounds.
    /// North-south neighbors return `None` when `r` would go below `0`
    /// or reach `height`.
    ///
    /// # Examples
    ///
    /// ```
    /// use coords::{Coord, Direction, MapBounds};
    /// let bounds = MapBounds::new(10, 10);
    ///
    /// // Normal neighbor
    /// assert_eq!(
    ///     bounds.neighbor(Coord::new(5, 5), Direction::East),
    ///     Some(Coord::new(6, 5)),
    /// );
    ///
    /// // East wraps
    /// assert_eq!(
    ///     bounds.neighbor(Coord::new(9, 5), Direction::East),
    ///     Some(Coord::new(0, 5)),
    /// );
    ///
    /// // North pole returns None
    /// assert_eq!(
    ///     bounds.neighbor(Coord::new(5, 0), Direction::NorthEast),
    ///     None,
    /// );
    /// ```
    #[must_use]
    pub fn neighbor(self, coord: Coord, direction: Direction) -> Option<Coord> {
        let (dq, dr) = direction.offset();
        let raw = Coord::new(coord.q + dq, coord.r + dr);
        self.normalize(raw)
    }

    /// Returns all in-bounds neighbors of `coord`.
    ///
    /// The returned array has one slot per [`Direction`] in
    /// [`Direction::ALL`] order. Slots for neighbors that fall off the
    /// north or south edge are `None`; east-west neighbors are always
    /// `Some` because they wrap.
    ///
    /// # Examples
    ///
    /// ```
    /// use coords::{Coord, MapBounds};
    /// let bounds = MapBounds::new(10, 10);
    /// let neighbors = bounds.neighbors(Coord::new(5, 5));
    /// // Interior tile: all 6 neighbors present
    /// assert!(neighbors.iter().all(|n| n.is_some()));
    /// ```
    #[must_use]
    pub fn neighbors(self, coord: Coord) -> [Option<Coord>; 6] {
        let mut result = [None; 6];
        for (i, dir) in Direction::ALL.iter().enumerate() {
            result[i] = self.neighbor(coord, *dir);
        }
        result
    }

    /// Returns the wrap-aware distance between two in-bounds coordinates.
    ///
    /// Uses cube coordinate distance, with the `q` delta adjusted for
    /// the shortest path around the cylindrical wrap seam.
    ///
    /// # Examples
    ///
    /// ```
    /// use coords::{Coord, MapBounds};
    /// let bounds = MapBounds::new(10, 10);
    ///
    /// assert_eq!(bounds.distance(Coord::new(0, 0), Coord::new(3, 0)), 3);
    ///
    /// // Wrap-aware: going west from 0 to 9 costs 1, not 9
    /// assert_eq!(bounds.distance(Coord::new(0, 5), Coord::new(9, 5)), 1);
    /// ```
    #[must_use]
    pub fn distance(self, a: Coord, b: Coord) -> i32 {
        let w = self.width.cast_signed();
        let dq_raw = (a.q - b.q).abs();
        // Wrap: shortest path around the cylinder.
        let dx = dq_raw.min(w - dq_raw);
        let dz = (a.r - b.r).abs();
        // Third cube axis: y = -x - z, so |dy| = |dx + dz|.
        // dx and dz are both non-negative magnitudes. We want the
        // combination that gives the shortest path, which means we want
        // them to cancel where possible — i.e. opposite signs.
        // The minimum of (dx + dz) and |dx - dz| gives |dy| correctly.
        let dy = (dx - dz).abs();
        (dx + dz + dy) / 2
    }

    /// Returns all in-bounds coordinates within `steps` of `center`.
    ///
    /// Includes `center` itself (distance 0). Coordinates that would
    /// fall off the north or south edge are excluded; wrap-normalized
    /// east-west coordinates are included.
    ///
    /// # Examples
    ///
    /// ```
    /// use coords::{Coord, MapBounds};
    /// let bounds = MapBounds::new(10, 10);
    /// let tiles = bounds.range(Coord::new(5, 5), 1);
    ///
    /// // Center + 6 neighbors (interior tile, none clipped)
    /// assert_eq!(tiles.len(), 7);
    /// assert!(tiles.contains(&Coord::new(5, 5)));
    /// ```
    #[must_use]
    pub fn range(self, center: Coord, steps: i32) -> Vec<Coord> {
        let mut result = Vec::new();
        for dq in -steps..=steps {
            for dr in -steps..=steps {
                // Cube coordinate constraint: s = -q - r, and |s| <= steps.
                // Equivalent to: the three cube axes must all be within range.
                let ds = -(dq + dr);
                if ds.abs() > steps {
                    continue;
                }
                let candidate = Coord::new(center.q + dq, center.r + dr);
                if let Some(normalized) = self.normalize(candidate) {
                    result.push(normalized);
                }
            }
        }
        result
    }
}

impl fmt::Display for MapBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Coord, Direction, MapBounds};

    mod map_bounds {
        use super::*;

        #[test]
        fn new_stores_dimensions() {
            let b = MapBounds::new(240, 150);
            assert_eq!(b.width(), 240);
            assert_eq!(b.height(), 150);
        }

        #[test]
        #[should_panic(expected = "map width must be non-zero")]
        fn new_panics_on_zero_width() {
            let _ = MapBounds::new(0, 10);
        }

        #[test]
        #[should_panic(expected = "map height must be non-zero")]
        fn new_panics_on_zero_height() {
            let _ = MapBounds::new(10, 0);
        }

        #[test]
        fn display_formats_as_dimensions() {
            assert_eq!(MapBounds::new(240, 150).to_string(), "240x150");
        }
    }

    mod normalize {
        use super::*;

        #[test]
        fn in_bounds_coord_is_unchanged() {
            let b = MapBounds::new(10, 10);
            assert_eq!(b.normalize(Coord::new(5, 5)), Some(Coord::new(5, 5)));
        }

        #[test]
        fn q_wraps_east_past_width() {
            let b = MapBounds::new(10, 10);
            assert_eq!(b.normalize(Coord::new(10, 0)), Some(Coord::new(0, 0)));
            assert_eq!(b.normalize(Coord::new(11, 0)), Some(Coord::new(1, 0)));
        }

        #[test]
        fn q_wraps_west_past_zero() {
            let b = MapBounds::new(10, 10);
            assert_eq!(b.normalize(Coord::new(-1, 0)), Some(Coord::new(9, 0)));
            assert_eq!(b.normalize(Coord::new(-2, 0)), Some(Coord::new(8, 0)));
        }

        #[test]
        fn r_out_of_bounds_returns_none() {
            let b = MapBounds::new(10, 10);
            assert_eq!(b.normalize(Coord::new(0, -1)), None);
            assert_eq!(b.normalize(Coord::new(0, 10)), None);
        }
    }

    mod neighbors {
        use super::*;

        #[test]
        fn interior_tile_has_six_neighbors() {
            let b = MapBounds::new(10, 10);
            let ns = b.neighbors(Coord::new(5, 5));
            assert!(ns.iter().all(Option::is_some));
        }

        #[test]
        fn north_edge_clips_northward_directions() {
            let b = MapBounds::new(10, 10);
            let ns = b.neighbors(Coord::new(5, 0));
            // NorthEast and NorthWest go to r=-1, which is out of bounds
            assert_eq!(ns[2], None); // NorthEast
            assert_eq!(ns[3], None); // NorthWest
        }

        #[test]
        fn east_edge_wraps_east_neighbor() {
            let b = MapBounds::new(10, 10);
            assert_eq!(
                b.neighbor(Coord::new(9, 5), Direction::East),
                Some(Coord::new(0, 5))
            );
        }

        #[test]
        fn west_edge_wraps_west_neighbor() {
            let b = MapBounds::new(10, 10);
            assert_eq!(
                b.neighbor(Coord::new(0, 5), Direction::West),
                Some(Coord::new(9, 5))
            );
        }
    }

    mod distance {
        use super::*;

        #[test]
        fn adjacent_tiles_are_distance_one() {
            let b = MapBounds::new(10, 10);
            for dir in Direction::ALL {
                if let Some(neighbor) = b.neighbor(Coord::new(5, 5), dir) {
                    assert_eq!(b.distance(Coord::new(5, 5), neighbor), 1);
                }
            }
        }

        #[test]
        fn distance_is_symmetric() {
            let b = MapBounds::new(10, 10);
            let a = Coord::new(2, 3);
            let c = Coord::new(7, 6);
            assert_eq!(b.distance(a, c), b.distance(c, a));
        }

        #[test]
        fn wrap_seam_takes_short_path() {
            let b = MapBounds::new(10, 10);
            // q=0 and q=9 are adjacent on a width-10 map
            assert_eq!(b.distance(Coord::new(0, 5), Coord::new(9, 5)), 1);
        }

        #[test]
        fn same_tile_is_distance_zero() {
            let b = MapBounds::new(10, 10);
            assert_eq!(b.distance(Coord::new(4, 4), Coord::new(4, 4)), 0);
        }
    }

    mod range {
        use super::*;

        #[test]
        fn range_zero_returns_only_center() {
            let b = MapBounds::new(10, 10);
            let tiles = b.range(Coord::new(5, 5), 0);
            assert_eq!(tiles.len(), 1);
            assert!(tiles.contains(&Coord::new(5, 5)));
        }

        #[test]
        fn range_one_interior_returns_seven_tiles() {
            let b = MapBounds::new(10, 10);
            let tiles = b.range(Coord::new(5, 5), 1);
            assert_eq!(tiles.len(), 7);
        }

        #[test]
        fn all_tiles_in_range_are_within_distance() {
            let b = MapBounds::new(10, 10);
            let center = Coord::new(5, 5);
            let steps = 3;
            for tile in b.range(center, steps) {
                assert!(b.distance(center, tile) <= steps);
            }
        }

        #[test]
        fn range_at_north_pole_clips_out_of_bounds() {
            let b = MapBounds::new(10, 10);
            let tiles = b.range(Coord::new(5, 0), 1);
            // Some northward neighbors are clipped
            assert!(tiles.len() < 7);
            // But all returned tiles are valid
            assert!(tiles.iter().all(|c| b.contains(*c)));
        }
    }
}
