//! Wrap-aware hexagonal coordinate math.
//!
//! Coordinates are represented in axial form (`q`, `r`). The east-west
//! axis wraps toroidally — coordinates that move off one edge reappear on
//! the other. The north-south axis is bounded — operations that would move
//! off the top or bottom of the map return `None`.
//!
//! All spatial operations in `worldgen` and `game` are built on this crate.
//! Neither crate depends on the other through this one; `coords` has no
//! dependencies of its own.
//!
//! See `docs/design/overview.md` §1.4 for the coordinate system rationale.

use std::fmt;

/// An axial hexagonal coordinate.
///
/// `q` runs along the wrap axis (east-west); `r` runs along the bounded
/// axis (north-south). A `Coord` carries no map context — it is the pure
/// integer pair. Wrap and bounds questions are answered by combining a
/// `Coord` with a [`MapBounds`].
///
/// Values may be negative or exceed any map's extents; this is normal
/// during intermediate calculations (e.g. computing a neighbor before
/// normalizing it back into bounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord {
    /// Position along the wrap (east-west) axis.
    pub q: i32,
    /// Position along the bounded (north-south) axis.
    pub r: i32,
}

impl Coord {
    /// Constructs a coordinate from raw axial components.
    ///
    /// Performs no validation; the returned coordinate may be outside
    /// any particular map's bounds. Use [`MapBounds`] operations to
    /// normalize or check.
    #[must_use]
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.q, self.r)
    }
}

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
}

impl fmt::Display for MapBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod coord {
        use super::*;

        #[test]
        fn new_stores_components_verbatim() {
            let c = Coord { q: 3, r: 4 };
            assert_eq!(c.q, 3);
            assert_eq!(c.r, 4);
        }

        #[test]
        fn equality_is_componentwise() {
            assert_eq!(Coord::new(1, 2), Coord::new(1, 2));
            assert_ne!(Coord::new(1, 2), Coord::new(2, 1));
        }

        #[test]
        fn display_formats_as_pair() {
            assert_eq!(Coord::new(3, -7).to_string(), "(3, -7)");
        }
    }

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
}
