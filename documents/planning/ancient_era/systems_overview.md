# Systems Overview — Ancient Era Core

**Last updated:** 2026-06-23 (S1 closed — supply pipeline systems sketched; combat next)

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

> **Doc sync — 2026-06-23.** The wall/road reversal is now fully propagated. ADRs
> 0033/0034/0035 are written and ADR-0032 is reversed (see *ADR coverage* below);
> **combat.md §3.10** (composed per-assault defense) and **workers.md §3/§9** (tile
> roads, `(tile, facing)` walls) are synced; **ADR-0028**'s stale
> outpost-construction parenthetical is fixed in place (still `Proposed`).
> **workers.md §5 was not stale** — its wall-cost text was already correct, so the
> earlier "§3, §5, §9" flag is corrected to §3/§9.
>
> **Remaining repo action:** delete the superseded duplicate
> `documents/adr/0032-per-edge-fortification.md`; the live wall ADR is
> `documents/adr/0032-directional-wall-segments.md`. Separately, the ADR index
> `documents/adr/README.md` required by documentation-guide §4.6 does not yet exist
> and should be created.

---

## Settled

### Ownership & trade networks
*Sketched in `empire.rs`, `transit.rs`, `settlement_economy.rs`. Rationale in chat;
**ADR pending** (Empire owner model + trade-network partition) — write in Claude Code.*

- ✅ **Empire is the only owner.** `Owner(EmpireId)` (dense `u8`, fixed roster) tags
  settlements, units, and infrastructure — replaces `SettlementOwner(PlayerId)`. No
  `Player` in authoritative state: a *civilization* (ADR-0022 identity) and a
  *controller* (human/AI, `ControllerMode`) are attributes of an empire, not owners.
  Control switching is a command; the client↔empire mapping is session-layer and does
  not replay.
- ✅ Empire entity owns `EmpireId`, `Treasury` (empire-wide gold/science/culture — the
  home those goods previously lacked), `RoadPool` (its own component; was "player
  entity"), and `ControllerMode`. A new **leaf module `empire`** holds the shared keys
  so `economy` and `transit` use them without depending on each other.
- ✅ **Trade network = a cached component partition** — not a graph, topology, or
  weighted structure. Each settlement carries a `TradeNetworkId` (a within-turn label,
  **not** an entity id — named to dodge the `NetworkId`/ADR-0024 collision). Pooling
  groups members by equal label and sums their `Storage` at spend time.
- ✅ **Recompute, not incremental.** `transit::recompute_trade_networks` flood-fills the
  affected empire's road tiles on a connectivity command (route finished, road
  destroyed, settlement founded/destroyed/captured) — never per turn. Merge and
  severance are the *same* relabel; no union-find, no stale-partition desync class
  (pillar 0.5). Visits *all* the empire's settlements (isolated = singleton).
- ✅ **No "find nearest network" search.** Settlement draw reads its own `TradeNetworkId`
  (O(1)); field draw reads the source `supply_source_in_range` returns, then that
  source's tag. The only distance traversal is field-supply (deferred, W7), never the
  partition.
- ✅ **Surplus distribution** (ADR-0030): whole-component, demand-first, then fill member
  storages by a per-settlement 0–10 `Storage::intake_priority` (descending); drain
  ascending. No distance or geography — a sort, not a traversal.

### Turn loop & resolution
*Sketched in `turn.rs` (loop) and `setup.rs` (`state_0`). Rationale in chat;
**ADR pending** (turn model + resolution order) — write in Claude Code.*

- ✅ **One state, two phases.** `Interactive` (controllers emit commands that
  complete *live*, world frozen) → lock → `Resolving` (AI emits via the same path,
  then the world advances one tick). No second "next-turn" state object.
- ✅ **Canonical command log = sole order of record** (pillar 0.4). Server assigns a
  `CommandSeq` on arrival; clients/replays re-apply the log, never re-derive order.
  Arrival order *is* interactive contention adjudication (a taken tile is invalid,
  not a conflict) — so the interactive phase needs no conflict rules. Per-empire FIFO
  falls out for free.
- ✅ **Three modes (simultaneous / sequential / mixed) are input-gating policies on
  one loop**, not three engines — who may submit when. Same apply path, log, advance.
  Mixed clusters on *contention this turn*, not the war flag (deferred with combat).
- ✅ **Simultaneity seam lives in combat only** (`turn::resolve_combat`), resolved in
  the advance against the locked snapshot — not in the interactive phase.
- ✅ **World-advance order:** produce (network-local → `Storage`, empire-wide →
  `Treasury`) → consume demand-first per pooled component (upkeep, construction via
  `advance_construction`/`advance_worker_jobs`, growth) → distribute surplus by
  `intake_priority` → **combat (last)** → tick. Combat-last ⇒ no mid-advance partition
  relabel (its connectivity changes fire an event-recompute for next turn).
- ✅ **Pool/distribute is an `economy` system**, not `transit`: the partition is
  materialized as per-settlement `TradeNetworkId` tags, so the reduce groups by tag
  and needs no `transit` import. (Resolves S1's "where does distribution live".)
- ✅ **Singleplayer = degenerate single-source case** of this loop; integrated-server
  (Stages 4–6) reshapes nothing.
- ✅ **Supply pipeline (S1) closed.** The economy phase systems are sketched
  (`produce_yields` → `resolve_supply` → growth/`distribute_surplus`). Demand-order
  decisions pinned: **food upkeep before growth**; **materials by queue priority then
  ascending settlement key** when short; the per-component pool is within-advance
  scratch, never stored. Starvation *selection* and the step-2-completion-in-combat
  question are deferred by reference (ADR-0031 / §2.6, and S7).

### Settlement economy & production
*Captured in `settlement_economy.rs` sketch; ADR-0028 (staffed-work model), ADR-0033
(economy runtime + priority allocation).*

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
*Captured in `jobs.rs`, `transit.rs`; ADR-0034 (settlement job system), ADR-0035
(transit + field supply).*

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
*Roads/rivers captured in ADR-0035 (tile roads); walls in ADR-0032 revised
(`(tile, facing)`). combat.md §3.10 and workers.md §3/§9 synced 2026-06-23.*

- ✅ Roads are **tile** infrastructure (`infrastructure: Field<u8>`), not edge
  state. Rivers are a tile property. Bridges are tile infrastructure (Classical+).
- ✅ Road jobs store **only endpoints**; the tile path is derived on demand by
  deterministic A* preferring existing roads. Built road tiles live in tile state
  (no stored path; auto-reroutes; resume re-derives the same continuation).
- ✅ Walls are one-sided **owned `(tile, facing)`** segments, not an edge
  primitive — built as queue jobs holding no job-slots and no district capacity.
  *(Decision settled, ADR-0032 revised; combat.md §3.10 + workers.md §3/§9 synced.)*

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

## ADR coverage — decisions now captured

These decisions were once recorded only in chat (the highest rationale-loss risk at
handoff). Each now has an ADR carrying its *why*; this table is the index from
decision to record. All Ancient-era ADRs are `Proposed` (none yet moved to
`Accepted`).

| Decision | Captured in |
|----------|-------------|
| Priority-ratio goal · greedy-with-bundles allocator · building-level completion bonus · off-the-top population split · `enabled` throttle | **ADR-0033** (settlement economy runtime + allocation) |
| Job system: site-owned jobs, one-per-job, two drivers, `ConstructionQueue`/`WorkerQueue`, promotion migration | **ADR-0034** (settlement job system) |
| Transit / field-supply: tile roads, derived paths, range-limited field supply vs. connectivity-wide settlement goods, worker materials buffer, shared materials pool | **ADR-0035** (transit and field supply) |
| Walls = one-sided owned `(tile, facing)`, not edge state | **ADR-0032** revised (`0032-directional-wall-segments.md`) |
| Entity-vs-owned-data (`struct` ≠ `Entity`; districts/buildings are data) | **ADR-0033** (Decision; extends the ADR-0023 tile-layer entity rule) |

---

### Major slices (return to these)

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| S1 | **Per-turn supply pipeline** — economy phase-system bodies in the resolution order. | ✅ | **Closed.** `economy::{produce_yields, resolve_supply, apply_growth}` + pure `distribute_surplus` sketched (bodies `todo!()`). Per component: pool (within-advance scratch) → upkeep → construction → growth → surplus by `intake_priority`. **Food: upkeep before growth** (army drags growth — intended). **Materials when short: queue priority, then ascending settlement key** (cross-settlement fairness = revisitable tuning). Deferred *by reference*: starvation selection (ADR-0031 / `cities.md` §2.6); "may a step-2 completion fight in step-4" → S7. Co-designed with S2. |
| S2 | **Network-scoped saturation signal** feeding `allocate()` | ⛔ S1 | The stockpile-feedback idea: per-network marginal-value per good, **two-tier** (cover throughput demand, then buffer to cap), smooth curve (avoid oscillation), computed from **start-of-turn** state (acyclic). Co-designed with S1. |
| S3 | **Worker unit module** — assignment, claim-at-assignment (pre-travel), orphan-revert, one-per-job. | 🔧 | Types + driver-system *signatures* sketched in `worker.rs`; system **bodies** are `todo!()`. Travel reuses general unit movement. |
| S4 | **Intent-level worker tasking** ("develop this outpost toward plan X" → sequences jobs). | 🌿 | **Hard requirement**, not optional: the construction reversal is only a UI win if worker tasking is intent-level, not per-building micro (§0.6). |
| S6 | **AI strategist (layer 3)** — goal-setting + build/expansion search over a horizon. | 🌿 | Command source; uses `allocate()` as inner loop; non-deterministic search permitted (emits commands through the validation gate). Replay/test reproducibility is a deliberate choice to make. |
| S7 | **Combat model** — the simultaneity policy at `turn::resolve_combat` (advance step 4). | 🌿 | **Gates** the advance's combat step and the mixed-mode initiative path; prerequisite for any combat threading. Rule-adjudicated (simultaneous) vs initiative-within-conflict-cluster (sequential/mixed). Reads the locked snapshot ("parallelize the map, serialize the reduce"). |
| S8 | **Game setup → `state_0`** — consume `WorldData`, spawn empires/settlements/units, seed partition. | 🔧 | Shape sketched in `setup.rs` (system signatures, `GameSetup`/`EmpireSetup`). **Content gated:** settlement placement on Stage 3.6 (starting positions, unbuilt); opening unit loadout on tech-tree + create/upkeep cost tuning. |

*(S5 — walls/edge-state doc sync — resolved 2026-06-23; ADR-0032 reversed + design
docs synced. See Settled → Spatial representation. ID retired.)*

### Worker supply & transit cost (sub-thread of S3)

| ID | Stub | Status | Resume notes |
|----|------|--------|--------------|
| W1 | Worker supply = §3.11 buffer, **positional refill**. | ✅ | Far from network = can't refill; near = refills from network. |
| W2 | Roads **extend** supply range, finite, tech-increasable. | ✅ | Range re-anchors at any supply **source** (city or relay outpost), measured along roads — this is what makes "trade outpost" chains the way to project reach. Field-supply (range-limited) is a distinct layer from ADR-0030 settlement-goods (connectivity-wide); hold them apart. |
| W3 | Worker's §3.11 **supply category = building materials** (its "ammo"). Building consumes it; in range it refills from the network (like outpost construction); out of range it runs from a finite reserve → stall → retreat. | ✅ | `MaterialsSupply` in `worker.rs`. One buffer = both the **cost** (materials drawn) and the **friction** (shuttle when far). Covers *all* worker construction. Near-binary full-rate / stall-on-empty (replaced the smooth-degrade idea). |
| W4 | Worker create/upkeep cost = tuning lever; **buffer size** = how deep past the network a worker can push. | 🌿 | Tuning, deferred. |
| W5 | Materials is the *only* build-relevant buffer; food = ambient unit upkeep. | ✅ | Confirmed materials-only. Food stays the universal §3.11 rule (attrition if stranded), not a per-worker concern. |
| W6 | Foraging: **deferred, not deleted.** | ✅ | Ancient surfaces a single slow out-of-supply rate, no "foraging" jargon (everything forages → no contrast). §3.11 unchanged; the faster can't-forage rate appears with industrial units. |
| W7 | **Field-supply coverage cache shape:** per-source reachable set with range-scoped invalidation — **not** a per-tile field. | 🌿 | Shape pinned, build deferred to unit-actions. A source's set is re-walked only when a road/source/relay *within its range* changes, so a quiet turn re-walks nothing; group orders compute coverage once per empire, then O(1) per consumer. Avoids the per-tile push-model that tempts full-map repaint. |

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
