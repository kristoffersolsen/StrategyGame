//! Transit: the route layer (roads now; rail, sea, air in later eras).
//!
//! Intended path `game::sim::transit`. Owns route jobs and the connectivity and
//! field-supply-range queries that the supply network and worker resupply read. A
//! route is *not* a settlement build target — it belongs to no settlement and may
//! connect arbitrary tiles, including a stray pass road built for movement rather
//! than trade.
//!
//! Road jobs share only the `Available` / `InProgress` lifecycle with the
//! settlement job queues (`crate::sim::settlement::jobs`); payload, ownership, and
//! pathing all differ, so the parallel is intentional and not unified into a
//! shared type — see that module's note.

use bevy::prelude::Component;
use coords::Coord;

use crate::sim::network_id::NetworkId;

/// Stable id of a road job within a player's [`RoadPool`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RoadJobId(pub(crate) u32);

/// A road connecting two tiles, claimed by one worker.
///
/// Stores **only** the endpoints. The tile path is never stored — it is derived on
/// demand by [`derive_road_path`], which prefers existing roads, so already-laid
/// road tiles (which live in tile state) are reused and only the gaps remain. The
/// path is disposable and auto-reroutes when the network changes; a resuming
/// worker re-derives the same continuation. Endpoints are arbitrary tiles.
pub(crate) struct RoadJob {
    pub(crate) id: RoadJobId,
    /// One endpoint.
    pub(crate) from: Coord,
    /// The other endpoint.
    pub(crate) to: Coord,
    pub(crate) status: RoadJobStatus,
}

/// Claim state for a road job. Parallel to the settlement `JobStatus`,
/// deliberately not shared (see module docs).
pub(crate) enum RoadJobStatus {
    /// Queued and unclaimed.
    Available,
    /// A worker is building it.
    InProgress,
}

/// A player's empire-wide pool of road endpoint jobs.
///
/// Per player, not per settlement: a road belongs to no settlement. A worker
/// assigned to road-building claims the nearest unclaimed job (one per job). Lives
/// on the player entity.
#[derive(Component)]
pub(crate) struct RoadPool {
    /// Unclaimed and in-progress road jobs for this player.
    pub(crate) jobs: Vec<RoadJob>,
}

/// Derives the tile path for a road job: A* from `from` to `to` that prefers
/// existing road tiles, so a partially built road is reused and only the gaps are
/// returned.
///
/// Not stored anywhere — recomputed whenever a worker needs the next gap tile, so
/// it reroutes as the network changes. Determinism (pillar 0.4) requires a stable
/// tiebreak on equal-cost paths.
pub(crate) fn derive_road_path(from: Coord, to: Coord) -> Vec<Coord> {
    let _ = (from, to);
    todo!("deterministic A* preferring existing roads; return ordered tile path")
}

/// Whether `tile` is within field-supply range of `player`'s connected road
/// network, and from which source — the query worker resupply uses.
///
/// Supply propagates from a source (a city, or a relay outpost holding stock) along
/// connected roads up to a finite range that technology increases. This is the
/// **field-supply** layer (range-limited), distinct from ADR-0030 settlement-goods
/// distribution (connectivity-wide). Returns the supplying source if in range, so
/// chained relay outposts extend reach.
pub(crate) fn supply_source_in_range(tile: Coord, player: NetworkId) -> Option<NetworkId> {
    let _ = (tile, player);
    todo!("walk the connected network from sources within the tech-scaled range")
}
