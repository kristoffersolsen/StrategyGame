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

use crate::sim::empire::EmpireId;
use crate::sim::network_id::NetworkId;

/// Stable id of a road job within an empire's [`RoadPool`].
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

/// An empire-wide pool of road endpoint jobs.
///
/// Per empire, not per settlement: a road belongs to no settlement. A worker
/// assigned to road-building claims the nearest unclaimed job (one per job). Its
/// own component on the empire entity (composition, not nested into `Empire`), so
/// `transit` owns it without `empire` having to import `transit`.
#[derive(Component)]
pub(crate) struct RoadPool {
    /// Unclaimed and in-progress road jobs for this empire.
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

/// Whether `tile` is within field-supply range of `empire`'s connected road
/// network, and from which source — the query worker resupply uses.
///
/// Supply propagates from a source (a city, or a relay outpost holding stock) along
/// connected roads up to a finite range that technology increases. This is the
/// **field-supply** layer (range-limited), distinct from ADR-0030 settlement-goods
/// distribution (connectivity-wide, the [`recompute_trade_networks`] partition).
/// Returns the supplying source settlement (its [`NetworkId`] entity handle) if in
/// range, so chained relay outposts extend reach; the caller reads that source's
/// [`crate::sim::empire::TradeNetworkId`] to draw from the correct network.
///
/// Coverage is computed *per source* with range-scoped invalidation, **not** as a
/// per-tile field — the per-source reachable set is cached and re-walked only when
/// a road, source, or relay within its range changes, so a quiet turn re-walks
/// nothing. Group orders compute coverage once per empire and then read it O(1) per
/// consumer rather than running this BFS per unit. The build is deferred to the
/// unit-actions pass; this shape is pinned (systems_overview W7).
pub(crate) fn supply_source_in_range(tile: Coord, empire: EmpireId) -> Option<NetworkId> {
    let _ = (tile, empire);
    todo!("walk the connected network from sources within the tech-scaled range")
}

// ------------- Trade-network partition (ADR-0030) -------------

/// Recomputes one empire's trade-network partition, stamping every one of its
/// settlements with a [`crate::sim::empire::TradeNetworkId`].
///
/// Runs **on a connectivity-affecting command, for the affected empire(s) only** —
/// never per turn: a route finishing, a road being destroyed, and a settlement
/// being founded, destroyed, or captured (capture touches two empires). A flood
/// fill over that empire's road tiles (the `infrastructure` tile field, ADR-0035)
/// gives connected components; every settlement center sharing a component gets the
/// same label, and a settlement no road reaches is its own singleton — so the pass
/// must visit *all* of the empire's settlements, not only those a road touches.
///
/// **Recompute, not incremental.** Merge (a finished route, possibly crossing an
/// existing road into a new junction) and severance (a removed road that may or may
/// not split a component) are the *same* operation here — relabel from scratch.
/// That is deliberate: detecting whether a removal split a component is the same
/// reachability question as recomputing, union-find has no cheap delete, and a
/// maintained topology carries a stale-partition desync bug class a recompute
/// cannot have. The flood fill is negligible against the per-turn yield sweep, and
/// sparse roads only make it cheaper — so the edge-by-edge optimization waits for a
/// profiler that says this is hot (pillar 0.5).
///
/// Labels are **within-turn**: the partition need only be *correct*, never stably
/// named, so no id allocator and no cross-turn scheme. Determinism (pillar 0.4)
/// requires settlements be visited in a stable order (ascending settlement key).
pub(crate) fn recompute_trade_networks() {
    todo!("flood-fill the empire's road tiles; stamp every settlement's TradeNetworkId")
}
