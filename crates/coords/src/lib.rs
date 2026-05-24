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

mod bounds;
mod coords;

pub use bounds::MapBounds;
pub use coords::{Coord, Direction};
