//! Settlement job mechanism: priority queues advanced by two labor drivers.
//!
//! Intended path `game::sim::settlement::jobs`. A settlement holds one or two job
//! queues over the shared [`Job`] type:
//!
//! - [`ConstructionQueue`] — build-slot-driven, **city only**: the city's own
//!   districts, buildings, walls. Advanced by [`advance_construction`].
//! - [`WorkerQueue`] — worker-driven, every settlement: tile improvements, plus an
//!   outpost's districts/buildings/walls. Advanced by the worker driver in
//!   `crate::sim::worker`.
//!
//! Both are **parallel**: N build-slots, or N workers (one per job), advance N
//! jobs at once. Jobs are **site-owned** — the job carries its `invested`, so
//! pause, reassignment, and worker death never lose progress.
//!
//! On promotion/demotion a half-built construct **migrates** between the two
//! queues with its `invested` intact (both hold the identical [`Job`]); the city
//! hall — itself the last worker-built outpost building — is the switch point.
//!
//! Roads are deliberately **not** here: they have a different shape (endpoints, a
//! derived path) and different ownership, and live in the `transit` module. The
//! `Available` / `InProgress` lifecycle is duplicated there on purpose; if it ever
//! grows shared behavior, extract a `claimable` trait then — not now.

use bevy::prelude::Component;

use crate::sim::settlement::build_target::BuildTarget;
use crate::sim::settlement::economy::Yields;

/// Stable id of a job within its queue, so a worker's claim survives the reorder
/// or removal of other jobs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct JobId(pub(crate) u32);

/// One unit of settlement build work. Site-owned: the job, not the labor, carries
/// progress.
///
/// There is no job-*kind* enum — with roads moved to `transit`, every settlement
/// job is a single-site construct or improvement, so its fields are inlined.
pub(crate) struct Job {
    pub(crate) id: JobId,
    /// What is built and where it lands.
    pub(crate) target: BuildTarget,
    /// Total cost from the target type's spec.
    pub(crate) cost: Yields,
    /// Accumulated investment; on `invested >= cost` the result is applied to site
    /// state and the target's `BuildState` (`super::economy`) flips to `Complete`.
    /// A paused job keeps this and returns to `Available` — no refund.
    pub(crate) invested: Yields,
    pub(crate) status: JobStatus,
}

/// Claim state. Shared by both drivers; managed differently — a worker holds a
/// persistent claim across travel, a build-slot is simply in this turn's top-N.
pub(crate) enum JobStatus {
    /// Queued and unclaimed (progress may be non-zero — a paused job lands here).
    Available,
    /// Being advanced.
    InProgress,
}

/// A priority-ordered pool of settlement jobs; labor claims from the top.
///
/// Which `BuildTarget`s are admissible is enforced at enqueue time by the command
/// layer, not by the type (a [`ConstructionQueue`] rejects tile-feature targets).
pub(crate) struct JobQueue {
    /// Highest priority first.
    pub(crate) jobs: Vec<Job>,
}

/// A city's **build-slot-driven** queue — its own districts, buildings, walls.
///
/// Present only on cities. Build-slot capacity is derived from the city's enabled
/// production buildings (`super::economy`), not stored.
#[derive(Component)]
pub(crate) struct ConstructionQueue(pub(crate) JobQueue);

/// A settlement's **worker-driven** queue — tile improvements for any settlement,
/// plus an outpost's constructs. A city's own constructs go to
/// [`ConstructionQueue`] instead.
#[derive(Component)]
pub(crate) struct WorkerQueue(pub(crate) JobQueue);

// ------------- Build-slot driver -------------

/// Advances a city's [`ConstructionQueue`] by one turn of build-slot throughput.
///
/// The top-`throughput` `Available` jobs accrue investment drawn from network
/// materials; completed jobs are applied and removed. No persistent claim or
/// orphan-revert — build-slots are fungible and local. `throughput` is the staffed
/// build-slot count computed by the economy (the population split lives there).
pub(crate) fn advance_construction(queue: &mut ConstructionQueue, throughput: u32) {
    let _ = (queue, throughput);
    todo!("advance top-N Available by throughput; apply + remove completed")
}
