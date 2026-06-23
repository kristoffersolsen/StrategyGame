# Systems Overview — Ancient Era Core

**Last updated:** 2026-06-22 (handoff: stale-docs flag + ADR gaps)

A living index of the Ancient-era core-systems design work: what is settled
enough to build, and what stubs we have deliberately deferred and must return
to. Updated as decisions land and new branches open.

This is **not** a design doc. The rationale for each decision lives in the
design docs and ADRs referenced here; this file only tracks *state* and *where
to resume*. When a stub is resolved, move it up to Settled with a pointer to
where it was captured.

**Status legend:** ✅ Settled (decided + sketched, safe to build) · 🔧 Decided,
reorg/impl pending · 🌿 Open (deferred, must return) · ⛔ Blocked (waits on
another item).

> ⚠️ **Known-stale docs — do not trust as written.** The ADR layer is now synced
> (ADR-0032 reversed to one-sided `(tile, facing)` walls; ADR-0033/0034/0035 added).
> What remains stale are **design docs and one ADR cross-ref** still describing
> reversed models (tracked as **S5** plus below). A cold read of these gives the
> *wrong* model; sync before building walls/roads.
> - **combat.md §3.10** — still describes shared per-edge wall defense; should be the
>   composed per-assault model (ADR-0032 revised).
> - **workers.md §3, §5, §9** — still frame roads/walls as edge state; roads are tile
>   infrastructure and walls are `(tile, facing)` (ADR-0035, ADR-0032 revised).
> - **ADR-0028** — its parenthetical "outpost construction is performed by a
>   road-connected city" is the pre-reversal model, superseded by ADR-0030 (revised);
>   fix in 0028's review pass.

---

## Settled

### Settlement economy & production
*Captured in `settlement_economy.rs` sketch; ADR-0028 (staffed-work model).*

- ✅ Settlement is the entity; city and outpost are the same entity at different
  center tiers (tier derived from the center content handle, no `is_city` flag).
- ✅ Districts and buildings are **owned data** on the settlement (nested
  `Vec<District>` → `Vec<Building>`), not entities. Reached through their owner,
  not iterated as a population.
- ✅ Yields are integer; `YieldKind` is a closed enum (Food, Materials, Science,
  Culture, Gold); multipliers are integer permille. No floats on the production
  path (determinism, pillar 0.4).
- ✅ One unified slot list: claimed-tile improvements carry job-slots on the same
  terms as building slots. A slot may produce multiple yields at once.
- ✅ Player directive is **priority ratios** (sum to a fixed total), not absolute
  targets. Allocation drives realized yields toward the priority distribution.
- ✅ Completion bonuses are **building-level only** (all of a building's slots
  staffed). District-level was rejected (bundle size would grow with footprint).
- ✅ Allocation = **greedy-with-bundles**: candidates are single-slot and
  whole-building-completion moves, scored by priority-weighted marginal yield
  against the running total; deterministic (integer, stable tiebreak); a pure
  function reusable as the AI search inner loop.
- ✅ `enabled` flag per building = the player's build-channel throttle, the mirror
  of priorities for the yield channel.
- ✅ Authoritative record vs. derived index: the nested record is authoritative
  for economy; `claimed: Vec<Coord>` is a settlement→tiles index; tile ownership
  and improvement identity stay authoritative on the dense grid (ADR-0023).

### Construction model
*Captured in revised ADR-0030; doc sync done across cities.md, workers.md,
ancient-tech-tree.md.*

- ✅ **Construction reversal:** workers build everything outside a city —
  including outpost district footprints *and* their buildings. The earlier
  "road-connected city constructs outpost buildings remotely" model was reversed
  (ADR-0030 revised in place; rejected alternative flipped).
- ✅ Two-axis rule: tile features (improvements, roads, bridges) are always worker
  work; settlement constructs (districts, buildings, walls) are build-slot work
  in a **city**, worker work in an **outpost**.
- ✅ City constructs are build-slot-only, **structurally** (not via tuning) — one
  worker-cost lever can't price two roles, and worker-built city constructs would
  let worker-spam bypass the build-slot economy.
- ✅ City-center bootstrap slot **places city districts** (the path kept; an
  earlier "always-worker district placement" proposal was rejected for the same
  spam reason).
- ✅ Build-slots claim population **off the top** ("building a workshop is the
  instruction to staff it"); throttle via `enabled`. **No** automatic material
  gate — draining the stockpile is a player-owned, UI-warned failure state
  (auto-gating would neuter supply-line stakes).
- ✅ Jobs are **site-owned**, not worker-owned: progress survives pause,
  reassignment, and worker death. Pause = revert to Available, no refund (same
  rule as severance).

### Job system
*Captured in `jobs.rs` (settlement) and `transit.rs` (roads).*

- ✅ Settlement jobs use one `Job` type (Site-shaped: `target` + `cost` +
  `invested`; no kind enum after roads split out), in a shared `JobQueue`.
- ✅ Two settlement queue roles: `ConstructionQueue` (build-slot driver, city
  only) and `WorkerQueue` (worker driver, every settlement). Both parallel-claim;
  the build-slot driver is the degenerate (no travel, no death) case.
- ✅ Roads are a **separate** system in `transit` (`RoadJob` / `RoadPool`,
  per-player, empire-wide), sharing only the Available/InProgress lifecycle —
  deliberately not unified into a shared type (X5).
- ✅ **One labor unit per job** (no stacking; prevents the worker-spam speed-up).
- ✅ Jobs are **site-owned**; progress survives pause, reassignment, worker death.
- ✅ Deterministic claim order: ascending worker `NetworkId`.
- ✅ Promotion/demotion migrates a half-built construct between the two settlement
  queues with `invested` intact (both hold the identical `Job`).

### Spatial representation
*Roads/rivers captured across ADR-0030, workers.md; walls decided in discussion
but docs not yet synced (see stubs).*

- ✅ Roads are **tile** infrastructure (`infrastructure: Field<u8>`), not edge
  state. Rivers are a tile property. Bridges are tile infrastructure (Classical+).
- ✅ Road jobs store **only endpoints**; the tile path is derived on demand by
  deterministic A* preferring existing roads. Built road tiles live in tile state
  (no stored path; auto-reroutes; resume re-derives the same continuation).
- ✅ Walls are one-sided **owned `(tile, facing)`** segments, not an edge
  primitive — built as queue jobs holding no job-slots and no district capacity.
  *(Decision settled; doc sync is an open stub.)*

### Module organization (executed)
*Reorg done; dead duplicates deleted. Realized sketch files:*
- ✅ `settlement::economy` (`settlement_economy.rs`) — production record, slots,
  allocation, `BuildState`.
- ✅ `settlement::build_target` (`build_target.rs`) — `BuildTarget` (Building,
  WallSegment, Improvement, DistrictFootprint), `DistrictId`. Road/Bridge removed.
- ✅ `settlement::jobs` (`jobs.rs`) — `Job` (Site-shaped, no kind enum now roads
  are gone), `JobQueue`, `ConstructionQueue`, `WorkerQueue`, `advance_construction`
  (build-slot driver).
- ✅ `transit` (`transit.rs`) — `RoadJob`, `RoadPool`, `derive_road_path`,
  `supply_source_in_range`. Own lifecycle, deliberately not unified with settlement
  jobs.
- ✅ `worker` (`worker.rs`) — `Worker`, `WorkerAssignment`, `JobSource`,
  `ClaimedJob`, `MaterialsSupply`, and the worker driver systems.
- ✅ Dead `BuildJob` / `SettlementConstruction` / `construction_demand` /
  `advance_queue` deleted.

---

## Decisions needing ADRs (rationale currently only in chat)

These are *decided and sketched*, but the **why** lives only in the originating
conversation — the highest rationale-loss risk at handoff. The sketches capture
the *what*; an ADR is needed for the *why* before the reasoning is unrecoverable.

| Decision | Rationale lives | ADR action |
|----------|-----------------|------------|
| Priority-ratio goal · greedy-with-bundles allocator · building-level completion bonus · off-the-top population split · `enabled` throttle | chat; partial in ADR-0028 | If ADR-0028 is still Proposed, revise it in place to add these; else a new ADR extending it. |
| Job system: site-owned jobs, one-per-job, two drivers, `ConstructionQueue`/`WorkerQueue`, promotion migration | chat; `jobs.rs` | **New ADR** (settlement job system). |
| Transit/field-supply: roads extend supply at finite tech-scaled range; field-supply (range-limited) vs ADR-0030 settlement-goods (connectivity-wide) as two layers; worker materials-supply category; transit competes for the materials pool | chat; `transit.rs`, `worker.rs` | **New ADR** (transit + field supply). |
| Walls = one-sided owned `(tile, facing)`, not edge state | chat | **Reverse ADR-0032** (this *is* the substance of S5). |
| Entity-vs-owned-data (`struct` ≠ `Entity`; districts/buildings are data) | chat; rule stated in ADR-0023 | Verify ADR-0023 captures the *why*; extend if thin. |

---

### Major slices (return to these)

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| S1 | **Supply network** — connected-component decomposition over road tiles; demand-first distribution; severance recalculation. | 🌿 | The next major slice. Owns connectivity (`transit`) → economy. |
| S2 | **Network-scoped saturation signal** feeding `allocate()` | ⛔ S1 | The stockpile-feedback idea: per-network marginal-value per good, **two-tier** (cover throughput demand, then buffer to cap), smooth curve (avoid oscillation), computed from **start-of-turn** state (acyclic). Co-designed with S1. |
| S3 | **Worker unit module** — assignment, claim-at-assignment (pre-travel), orphan-revert, one-per-job. | 🔧 | Types + driver-system *signatures* sketched in `worker.rs`; system **bodies** are `todo!()`. Travel reuses general unit movement. |
| S4 | **Intent-level worker tasking** ("develop this outpost toward plan X" → sequences jobs). | 🌿 | **Hard requirement**, not optional: the construction reversal is only a UI win if worker tasking is intent-level, not per-building micro (§0.6). |
| S5 | **Walls / edge-state doc sync** | 🌿 | Walls = `(tile, facing)`, roads = tile. Stale: ADR-0032 (titled "per-edge fortification"), combat.md §3.10 (shared per-edge defense), workers.md §3 (roads/walls as edge). Its own ADR-reversal + sync pass. |
| S6 | **AI strategist (layer 3)** — goal-setting + build/expansion search over a horizon. | 🌿 | Command source; uses `allocate()` as inner loop; non-deterministic search permitted (emits commands through the validation gate). Replay/test reproducibility is a deliberate choice to make. |

### Worker supply & transit cost (sub-thread of S3)

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| W1 | Worker supply = §3.11 buffer, **positional refill**. | ✅ | Far from network = can't refill; near = refills from network. |
| W2 | Roads **extend** supply range, finite, tech-increasable. | ✅ | Range re-anchors at any supply **source** (city or relay outpost), measured along roads — this is what makes "trade outpost" chains the way to project reach. Field-supply (range-limited) is a distinct layer from ADR-0030 settlement-goods (connectivity-wide); hold them apart. |
| W3 | Worker's §3.11 **supply category = building materials** (its "ammo"). Building consumes it; in range it refills from the network (like outpost construction); out of range it runs from a finite reserve → stall → retreat. | ✅ | `MaterialsSupply` in `worker.rs`. One buffer = both the **cost** (materials drawn) and the **friction** (shuttle when far). Covers *all* worker construction. Near-binary full-rate / stall-on-empty (replaced the smooth-degrade idea). |
| W4 | Worker create/upkeep cost = tuning lever; **buffer size** = how deep past the network a worker can push. | 🌿 | Tuning, deferred. |
| W5 | Materials is the *only* build-relevant buffer; food = ambient unit upkeep. | ✅ | Confirmed materials-only. Food stays the universal §3.11 rule (attrition if stranded), not a per-worker concern. |
| W6 | Foraging: **deferred, not deleted.** | ✅ | Ancient surfaces a single slow out-of-supply rate, no "foraging" jargon (everything forages → no contrast). §3.11 unchanged; the faster can't-forage rate appears with industrial units. |

*Transit-cost consequence:* worker construction now competes with build-slot
construction for the **same materials pool**, so road/outpost-spam is
self-limiting through the economy (no auto-gate; draining the stockpile stays
the player's problem). This resolves **P4** (roads cost materials, via the W3
buffer).

### Cross-cutting contracts & structures

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| X1 | **`Plan` cache invalidation contract** | 🌿 | Every writer (build completion, worker siting, population, goal edit, **and** network-signal change) sets `plan = None`. ADR-worthy discipline note. |
| X2 | **Tile-side derived indices** | 🌿 | `Field<Option<DistrictRef>>` and improvement index (coord → owner construct) for rendering/picking/adjacency; written by the command processor alongside the record. |
| X3 | **Per-settlement in-progress building records** (site-owned store for worker-built outpost buildings). | 🌿 | Not yet sketched; lives where worker assignment lives (S3). Distinct from the city build-slot queue. |
| X4 | **`Slot` source coord** | 🌿 | Improvement slots may need their tile (rendering "worked", adjacency, pillage removal). Pin at flattening-walk design. |
| X5 | Shared job lifecycle now duplicated across settlement queues and `RoadPool` — **intentional**; extract a `claimable` trait only if it grows. | 🌿 | One-line note in both module docs so it doesn't read as oversight. |
| X6 | `settlement_economy.rs` still uses pre-reorg `use` paths (`crate::sim::economy`, not `crate::sim::settlement::economy`). New files use target paths. | 🔧 | Cosmetic; reconcile when the real module tree is built. |

### Allocation tuning (deferred by design)

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| A1 | Exact integer scoring formula (priority-against-running-total). | 🌿 | Tuning; left out of the type sketch. |
| A2 | Priority-queue accelerator (O(S log S) per allocation). | 🌿 | Add when the AI goal-search makes `allocate()` a hot inner loop, not before. |
| A3 | Local-improvement pass (close greedy's threshold gap on small settlements). | 🌿 | Add only if playtesting shows manual beats auto by a tempting margin. |
| A4 | Completion-bonus **magnitude** tuning. | 🌿 | Controls both required algorithm sophistication and circumvention risk; keep modest. |

### Promotion / later content

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| P1 | Promotion/demotion mechanics: city hall = last worker-built outpost building; completion = labor-model switch + grants bootstrap slot. | 🌿 | In-progress-job migration policy (migrate-in-place vs finish-first) decided at wiring time. |
| P2 | Bridges + road-crosses-river interaction. | 🌿 | Classical+. Auto-place bridges on a road path's river tiles, or separate jobs? |
| P3 | Road tile ownership / usage (can enemies use your roads; player-tagged?). | 🌿 | Supply/combat detail. |
| P4 | Materials cost of roads/improvements (lumber, beyond worker-time). | ✅ → W3 | Resolved: roads/worker-construction draw network materials via the worker's materials buffer. |
| P5 | `BuildState` home → lives in `settlement::economy` (its hot reader and writers are both settlement-side). | ✅ | Resolved in the reorg; `BuildState` is in `settlement_economy.rs`. |

---

## How to maintain this file

- When a stub resolves: move it to **Settled** with a pointer to where it was
  captured (sketch file, design doc §, or ADR), and bump **Last updated**.
- When a new branch opens mid-discussion: add a stub row immediately so it isn't
  lost, with just enough resume context to pick it up cold.
- Keep entries terse — this indexes the work, it doesn't restate it.
