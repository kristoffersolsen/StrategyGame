# ADR 0034: Settlement job system — site-owned jobs, two labor drivers, one labor unit per job

**Status:** Proposed
**Date:** 2026-06-22
**Deciders:** Kristoffer

## Context

ADR-0028 defines build-slots (population consuming materials to produce a discrete
object) and ADR-0030 (revised) makes workers the sole labor for everything outside a
city. That leaves a runtime question both share: **how is queued construction
represented, claimed, and advanced**, for both build-slot work (a city's interior)
and worker work (improvements, and an outpost's districts/buildings), such that
parallel construction, the AI's need to prioritize a queue, and pause/resume all
work — without two parallel queue systems (pillar 0.5)?

A first design over-unified roads into the same queue; that was rejected (ADR-0035 —
roads have different payload, ownership, and pathing). What remains genuinely shared
is the *settlement* construction lifecycle. The conventional instincts (one giant
per-city build queue holding its outposts' work; worker-owned in-progress jobs;
allowing several workers to stack on one job for speed) were each challenged.

Constraints: pillar 0.4 (deterministic claim order), pillar 0.6 (the queue is the
intent-level control surface, edited by player and AI alike), and the worker-spam
exploit (cheap mobile labor must not be able to out-produce the build-slot economy).

## Decision

**One `Job` type, one `JobQueue`, two labor drivers.** A settlement job is a
single-site construct or improvement (`target`, `cost`, accumulated `invested`,
`status`) in a priority-ordered `JobQueue`. Two drivers advance queues; the queue is
identical:

- **Build-slot driver** — fungible *local* labor with no travel and no death. Each
  turn the city's available build-slot throughput advances the top-N `Available`
  jobs. Drives the city's `ConstructionQueue` (the city's own districts, buildings,
  walls). Present only on cities.
- **Worker driver** — mobile unit entities that claim a job, travel to it, advance
  it, and revert it on death/unassignment. Drives a settlement's `WorkerQueue` (tile
  improvements for any settlement, plus an outpost's districts/buildings/walls).
  Present on every settlement.

The build-slot driver is the degenerate case of the worker driver (no travel, no
orphan-revert); the same `JobQueue` serves both.

**Jobs are site-owned, not worker-owned.** The job — not the labor — carries
`invested`. A worker holds only an *assignment* referencing the job it advances.
Unassigning or killing the worker reverts the job to `Available` with progress intact
(no refund — the same rule as supply severance, ADR-0030), and any worker can resume
it. This is forced, not merely convenient: invested materials are physically at the
build site, not in the worker (ADR-0030 forbids background material transfer); tile
improvements are tile state (ADR-0023), so their in-progress state is naturally site
state; and site-ownership is what lets the player issue an intent-level directive
that *creates* jobs while worker assignment is the automatable layer (pillar 0.6).

**One labor unit per job.** A job is advanced by at most one worker (or counts as one
build-slot's work). Parallelism comes from having more workers / more build-slots, not
from stacking several onto one job — stacking would let worker-spam collapse build
timelines and recreate the exploit.

**Deterministic claim order.** When several idle labor units claim from one queue in a
turn, the order is ascending unit `NetworkId`: the top-priority `Available` job goes
to the lowest-id idle unit (pillar 0.4).

**Promotion/demotion migrates jobs.** A settlement construct's labor channel depends
on tier (build-slot in a city, worker in an outpost). The city hall is the last
worker-built outpost building; its completion is the switch point. A half-built
construct caught at the flip **migrates** between the `WorkerQueue` and the
`ConstructionQueue` with its `invested` intact, because both hold the identical
`Job`. (Migrate-in-place vs. let-leftovers-finish is a wiring-time policy, not a type
change.)

**Roads are excluded** and live in `transit` (ADR-0035). The `Available`/`InProgress`
lifecycle is duplicated there on purpose; extracting a shared `claimable` trait waits
until it grows real shared behavior (pillar 0.5).

## Consequences

**Positive:**

- One queue type and lifecycle for all settlement construction; the driver is the only
  thing that varies (pillar 0.5).
- Site-owned jobs make pause, reassignment, and worker death cost only time, not
  progress — and are the structural enabler of intent-level worker tasking.
- One-per-job makes worker count a legible throughput limit and closes the
  worker-spam exploit.
- The promotion migration is trivial because both queues hold the same `Job`.
- The same queues the player edits are what the AI fills — one control surface, two
  agents (pillar 0.6).

**Negative:**

- Two queue components on a city (construction + worker) rather than one. Accepted:
  they are fed by different labor pools, so a unified priority order would be
  meaningless (you cannot reprioritize a worker road ahead of a build-slot wall).
- Outposts need a site-owned store for in-progress worker-built building records,
  distinct from the city build-slot queue — flagged for the worker-module wiring.
- Deterministic-by-`NetworkId` claim can assign a distant job to a unit a nearer one
  could reach sooner; accepted for determinism, mitigated by the travel cost being
  visible.

## Alternatives considered

- **One centralized per-city build queue holding outpost work:** rejected (the
  pre-reversal model). A capital with several outposts holds one giant mixed queue
  serialized through its build-slots, and presents a confusing two-agent seam.
  Superseded by ADR-0030's worker-built exterior.
- **Worker-owned in-progress jobs:** rejected. Worker death would evaporate a
  multi-tile road or a half-built building, materials would have to live "in" the
  worker (against ADR-0030), and intent-level tasking would be impossible.
- **Several workers stacking on one job for a speed-up:** rejected — it reopens the
  worker-spam exploit and makes throughput illegible.
- **One fully-generic queue including roads:** rejected (ADR-0035). Roads differ in
  payload, ownership, and pathing; forcing them through one type is premature
  unification.

## References

- ADR-0023 (improvements are tile state; entity discipline).
- ADR-0028 (build-slots; the build-slot driver's labor).
- ADR-0030 (workers build the exterior; the no-refund severance rule).
- ADR-0033 (the economy record these jobs build into; build-slot population split).
- ADR-0035 (transit; why roads are a separate system).
- `cities.md` §2.7, §2.12 (construction model, promotion); `workers.md` §1 (worker
  scope).
- Design pillars 0.4 (determinism), 0.5 (no over-engineering), 0.6 (intent-level).
