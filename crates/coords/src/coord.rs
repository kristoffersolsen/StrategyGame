//! Axial hexagonal coordinate and direction types.

use std::fmt;

/// An axial hexagonal coordinate.
///
/// `q` runs along the wrap axis (east-west); `r` runs along the bounded
/// axis (north-south). A `Coord` carries no map context — it is the pure
/// integer pair. Wrap and bounds questions are answered by combining a
/// `Coord` with a [`crate::MapBounds`].
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
    /// any particular map's bounds. Use [`crate::MapBounds`] operations to
    /// normalize or check.
    /// /// # Examples
    ///
    /// ```
    /// use coords::Coord;
    /// let c = Coord::new(3, -7);
    /// assert_eq!(c.q, 3);
    /// assert_eq!(c.r, -7);
    /// ```
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

// ------------- Directions -------------

/// The six directions on a pointy-top hexagonal grid.
///
/// Directions are named by compass point. On a cylindrical map,
/// [`East`](Direction::East) and [`West`](Direction::West) wrap; the
/// remaining four directions are bounded at the north and south poles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// +q, same r.
    East,
    /// -q, same r.
    West,
    /// +q, -r.
    NorthEast,
    /// same q, -r.
    NorthWest,
    /// same q, +r.
    SouthEast,
    /// -q, +r.
    SouthWest,
}

impl Direction {
    /// All six directions in a fixed order.
    ///
    /// The order is stable across versions and may be relied upon by
    /// callers that iterate all neighbors.
    pub const ALL: [Direction; 6] = [
        Direction::East,
        Direction::West,
        Direction::NorthEast,
        Direction::NorthWest,
        Direction::SouthEast,
        Direction::SouthWest,
    ];

    /// Returns the axial `(q, r)` offset for this direction.
    #[must_use]
    pub(crate) const fn offset(self) -> (i32, i32) {
        match self {
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
            Direction::NorthEast => (1, -1),
            Direction::NorthWest => (0, -1),
            Direction::SouthEast => (0, 1),
            Direction::SouthWest => (-1, 1),
        }
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
}
