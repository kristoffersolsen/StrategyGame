//! Settlement build taxonomy: what a settlement can build.
//!
//! Intended path `game::sim::settlement::build_target`. This is the vocabulary
//! the job mechanism (`super::jobs`) operates over — *what* exists to build, as
//! opposed to *how* it is queued and advanced. Settlement-local: routes (roads,
//! bridges) are **not** here; they moved to the `transit` module, which owns its
//! own route jobs. `BuildState` (the live / under-construction status) lives with
//! the economy record in `super::economy`, since the yield sweep is its hot
//! reader.

use content::Handle as ContentHandle;
use coords::{Coord, Direction};

/// Stable per-settlement district identity.
///
/// A queued building names the district its result joins; a bare `Vec` index into
/// the economy record would be fragile across district removal. Implies a matching
/// `id` field on `District` in `super::economy`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct DistrictId(pub(crate) u16);

/// What a settlement build job produces on completion, and where it lands.
///
/// Settlement constructs and tile improvements only. Which labor channel advances
/// a target is decided by placement, not by the variant — build-slots for a city's
/// own constructs, workers for an outpost's and for all tile features (see
/// `super::jobs` and `crate::sim::worker`).
pub(crate) enum BuildTarget {
    /// A building constructed inside a district.
    Building {
        /// Which district the finished building joins. Settlement context is the
        /// queue owner for build-slot work, or the worker's target outpost for
        /// worker work.
        district: DistrictId,
        /// Building type spec.
        kind: ContentHandle,
    },
    /// A one-sided owned wall segment on a tile, facing one direction outward —
    /// not an edge primitive; a contested far side stores its own segment.
    WallSegment {
        /// The owned tile the wall sits on.
        tile: Coord,
        /// The outward direction the wall faces.
        facing: Direction,
    },
    /// A tile improvement (farm, mine) on a claimed tile.
    Improvement {
        /// The claimed tile improved.
        tile: Coord,
        /// Improvement type spec.
        kind: ContentHandle,
    },
    /// A district footprint placement on claimed land.
    DistrictFootprint {
        /// District type spec.
        kind: ContentHandle,
        /// Tiles the footprint will occupy.
        footprint: Vec<Coord>,
    },
}
