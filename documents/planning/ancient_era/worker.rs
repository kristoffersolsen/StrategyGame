//! Worker unit: exterior labor — assignment, claiming, and the materials buffer.
//!
//! Intended path `game::sim::worker`. A worker is the player's sole means of
//! changing tiles outside a city (tile improvements, roads, and an outpost's
//! districts/buildings/walls). It binds to one site-owned job, travels to it via
//! general unit movement, and advances it; the job's progress survives the worker,
//! so pause, reassignment, and death never lose work.
//!
//! # Materials supply (reuses combat.md §3.11)
//!
//! A worker's §3.11 supply category is **building materials** — its "ammunition".
//! Building consumes materials from the worker's buffer; when in field-supply range
//! of the network ([`crate::sim::transit::supply_source_in_range`]) the buffer
//! refills straight from network materials, demand-first, exactly as build-slot
//! construction draws them. Out of range the worker builds from its carried reserve
//! until empty, then retreats to refill. One buffer thus carries both the **cost**
//! (materials drawn from the pool, competing with build-slot construction) and the
//! **friction** (the shuttle when far from the network). Materials is the *only*
//! build-relevant buffer; food is ambient unit upkeep, not a second per-worker
//! pressure.

use bevy::prelude::Component;

use crate::sim::network_id::NetworkId;
use crate::sim::settlement::jobs::JobId;
use crate::sim::transit::RoadJobId;

/// Marker for worker units.
#[derive(Component)]
pub(crate) struct Worker;

/// A worker's standing assignment and the job it currently holds.
///
/// `source` is where the worker pulls jobs from; `claimed` is the specific job it
/// is advancing, recorded so death or unassignment can walk that job back to
/// `Available`. One worker per job. The claim is taken at assignment time, before
/// travel, so a second worker will not also head for the same site.
#[derive(Component)]
pub(crate) struct WorkerAssignment {
    /// Where the worker pulls its next job from.
    pub(crate) source: JobSource,
    /// The job currently held, if any.
    pub(crate) claimed: Option<ClaimedJob>,
}

/// Where a worker pulls its next job from.
pub(crate) enum JobSource {
    /// This settlement's `WorkerQueue` — claim the top `Available` job.
    Settlement(NetworkId),
    /// The player's `RoadPool` — claim the nearest unclaimed road job.
    Roads,
}

/// The specific job a worker is advancing, identified within its pool for revert.
pub(crate) enum ClaimedJob {
    /// A job in a settlement's `WorkerQueue`.
    Settlement {
        /// The settlement that owns the queue.
        settlement: NetworkId,
        /// The claimed job.
        job: JobId,
    },
    /// A job in the player's `RoadPool`.
    Road(RoadJobId),
}

/// The worker's building-materials supply buffer (its §3.11 supply category).
///
/// Drained by building, refilled from network materials when in field-supply
/// range. An empty buffer out of range is what forces a resupply retreat.
#[derive(Component)]
pub(crate) struct MaterialsSupply {
    /// Materials currently carried.
    pub(crate) current: u32,
    /// Maximum the worker can carry — the tuning lever for how far past the network
    /// a worker can build before it must retreat.
    pub(crate) capacity: u32,
}

// ------------- Worker driver -------------

/// Binds idle workers to `Available` jobs from their source, deterministically.
///
/// When several idle workers share a source in one turn, claim order is ascending
/// worker [`NetworkId`]: the top-priority `Available` job goes to the lowest-id
/// idle worker (pillar 0.4); for [`JobSource::Roads`], top priority is the nearest
/// unclaimed endpoint, ties broken by [`RoadJobId`]. Marks the job `InProgress` and
/// records it in [`WorkerAssignment::claimed`].
pub(crate) fn assign_idle_workers() {
    todo!("ascending NetworkId claims top Available / nearest road job; one per job")
}

/// Advances each worker's claimed job by one turn, spending materials.
///
/// A worker at its job's site spends building materials from [`MaterialsSupply`]
/// (with its worker-time) into the job's `invested`. On completion the result is
/// applied to site state and the job removed; if the buffer empties the worker
/// stalls and is routed to resupply, but the job stays `InProgress` with its
/// progress intact (site-owned). Travel to the site is general unit movement.
pub(crate) fn advance_worker_jobs() {
    todo!("spend MaterialsSupply into invested; complete, or stall and resupply")
}

/// Refills workers' [`MaterialsSupply`] when they are in field-supply range.
///
/// A worker within range of its player's connected network
/// ([`crate::sim::transit::supply_source_in_range`]) refills from network
/// materials, drawn demand-first — the same pool build-slot construction spends, so
/// heavy road-building competes with building construction. Out of range, no refill.
pub(crate) fn refill_worker_supply() {
    todo!("in-range workers draw network materials into MaterialsSupply")
}

/// Releases a claimed job back to `Available` when its worker dies or is
/// unassigned, so any other worker can resume it with progress intact.
pub(crate) fn release_orphaned_jobs() {
    todo!("on worker death/unassign, walk WorkerAssignment::claimed back to Available")
}
