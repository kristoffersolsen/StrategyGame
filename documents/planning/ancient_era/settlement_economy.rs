//! Settlement economy: the staffed-work production model (ADR-0028).
//!
//! A settlement — city or outpost, the same entity at different center tiers
//! (`cities.md` §2.12) — owns its full economic record: the districts it has
//! built, the buildings inside them, the player's priority directive, and the
//! cached population assignment. Claimed-tile improvements (`workers.md`) also
//! carry staffed slots, so the allocator runs over a *unified* flattened slot
//! view assembled from buildings and improvements together.
//!
//! # The read/write split this module is built around
//!
//! The authoritative state is the nested `districts` record. [`SlotTable`] and
//! [`Assignment`] are *derived* and cached, rebuilt only when the record,
//! population, or goal changes — never per turn. The per-turn sweep only reads
//! the assignment and sums yields ([`settlement_yields`]); re-allocation
//! ([`allocate`]) runs on change as a command-source computation, not in the
//! sweep.
//!
//! Two consequences of that split show up below as deliberate shapes:
//!
//! - **Liveness vs. progress.** [`BuildState`] is a cheap *status* the yield
//!   sweep branches on; construction *progress* (cost and accumulated
//!   investment) lives in the build subsystem, which is the sole writer of this
//!   status. The record holds the *result* of construction; the build system
//!   holds the *process*.
//! - **Record vs. tile index.** This record is authoritative for the economy
//!   but is never read by the renderer. Spatial and visual queries use a
//!   derived `Field<Option<_>>` tile index written alongside it; the presentation
//!   tier reads that index and reacts to events, never this record.
//!
//! This is a reviewable type sketch: function bodies are `todo!()`, and the
//! content *schemas* (building and district *type* definitions) are
//! registry-loaded RON, out of scope here.

use core::ops::Range;

use bevy::prelude::Component;
use content::Handle as ContentHandle;
use coords::Coord;

use crate::sim::network_id::NetworkId;

// ------------- Value types -------------

/// The fundamental yield axes — closed and slow-changing across all eras.
///
/// Unlike buildings or resources (content-registry data), the *set* of yield
/// kinds is core, so it backs a dense array with the same enum-indexed O(1)
/// discipline as the terrain yield table (ADR-0022). Network scope (food and
/// materials are network-local; gold, science, culture empire-wide, ADR-0030)
/// is a *distribution* concern, not a production one — every axis is produced
/// here and routed by scope downstream.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum YieldKind {
    /// Drives population growth and unit upkeep; network-local.
    Food,
    /// Building materials consumed by construction; network-local.
    Materials,
    /// Research output; empire-wide.
    Science,
    /// Cultural output; empire-wide.
    Culture,
    /// Currency; empire-wide.
    Gold,
}

impl YieldKind {
    /// Number of yield axes; the backing-array length for [`Yields`].
    pub(crate) const COUNT: usize = 5;
}

/// One integer amount per [`YieldKind`], indexed by discriminant.
///
/// Integer, not floating point, so all production math is deterministic
/// (pillar 0.4). Sub-unit scaling is applied in permille via integer
/// arithmetic, never as a float.
#[derive(Clone, Copy, Default)]
pub(crate) struct Yields([i32; YieldKind::COUNT]);

/// A multiplier in parts per thousand (`1000` == ×1.0). Integer for determinism.
#[derive(Clone, Copy)]
pub(crate) struct Permille(pub(crate) u32);

/// Settlement-global multipliers, one per axis.
///
/// Applied once to the staffed sum *after* completion bonuses, not per slot.
/// The Ancient instance is the farming center's settlement-wide food multiplier
/// (`cities.md` §2.12); slot-local boosts (adjacency, co-location) are *not*
/// here — they are computed from the record during the sweep.
#[derive(Clone, Copy)]
pub(crate) struct YieldMultipliers([Permille; YieldKind::COUNT]);

// ------------- Authoritative record (nested) -------------

/// A built district: a fixed multi-tile footprint holding buildings.
pub(crate) struct District {
    /// District type — housing / production / military / farming / mining — as a
    /// content handle (terrain rules ADR-0029, matching-boost rules ADR-0028).
    pub(crate) kind: ContentHandle,
    /// Tiles this footprint occupies. Small and fixed by era stamp (Ancient:
    /// 1 tile housing, 2 tiles working — ADR-0029). The source for the derived
    /// tile index the renderer reads.
    pub(crate) footprint: Vec<Coord>,
    /// Buildings constructed inside this district.
    pub(crate) buildings: Vec<Building>,
    /// Build status of the footprint placement itself.
    ///
    /// Accommodates both possible siting rules without deciding between them: if
    /// worker siting is instant the footprint is born [`BuildState::Complete`];
    /// if it costs worker-time it starts [`BuildState::UnderConstruction`].
    pub(crate) state: BuildState,
}

/// A building inside a district. Holds slots; may grant a completion bonus.
pub(crate) struct Building {
    /// Building type (house, mill, silo, furnace, barracks, library, …) as a
    /// content handle carrying its slot spec, completion bonus, build cost, and
    /// effects.
    pub(crate) kind: ContentHandle,
    /// Whether this building's slots are live (see [`BuildState`]).
    pub(crate) state: BuildState,
    /// Whether the player has this building active. A disabled building
    /// contributes *nothing* to the flattening walk — neither yield-slots nor
    /// build-slots — so it staffs no population and draws no materials. This is
    /// the player's lever over the build channel, the mirror of
    /// [`Goal::priorities`] over the yield channel, and finer than priorities
    /// even for yield buildings: it mothballs *one* building, where a zero
    /// priority unstaffs every building on that axis. Toggling it invalidates the
    /// cached [`Plan`]. Used for deliberate throttling (stockpiling, burst-prep);
    /// involuntary material shortage is handled automatically by the build-slot
    /// material gate, not by forcing the player to toggle this.
    pub(crate) enabled: bool,
}

/// Whether a built thing's slots are live yet — a *status summary*, not a
/// construction process.
///
/// The yield sweep reads this as one cheap branch: are this thing's slots
/// staffable or not. It is **not** the construction process — build cost and
/// accumulated progress live in the build subsystem, which is the sole writer of
/// this status and flips it to [`Complete`](BuildState::Complete) in the same
/// step it observes progress reaching cost. The yield path only ever reads it,
/// and is deliberately oblivious to which path did the building — build-slots, a
/// road-connected city's materials, or worker-time (ADR-0028, ADR-0030).
///
/// Keeping this derived-from-progress and single-writer avoids the
/// denormalization hazard of two facts that can disagree.
pub(crate) enum BuildState {
    /// Under construction; contributes no live slots and no yields.
    UnderConstruction,
    /// Finished; slots are live and staffable.
    Complete,
}

/// Per-settlement stock and capacity for network-local goods (ADR-0030).
///
/// Each settlement physically holds its own pile, which is what makes supply
/// severance a recalculation rather than a tug-of-war. Only the network-local
/// axes (food, materials) live here; the empire-wide axes (gold, science,
/// culture) live in [`crate::sim::empire::Treasury`] on the empire entity.
pub(crate) struct Storage {
    /// Current stock. Only network-local axes (food, materials) are meaningful.
    pub(crate) stock: Yields,
    /// Capacity = center base + silos. Overflow past all reachable capacity is
    /// lost (ADR-0030).
    pub(crate) capacity: Yields,
    /// Surplus-intake priority, `0..=10` (ADR-0030 surplus distribution). Once
    /// the whole trade network's demand (upkeep, growth, build draw) is met,
    /// the remainder fills member storages in *descending* priority; drain runs
    /// in *ascending* priority. `0` never accepts surplus and is drained first;
    /// `10` accepts maximally and is drained last. This governs **surplus only** —
    /// demand is always met before any intake, so it is never a consumption-order
    /// knob. The component-wide comparison is a sort, not a graph traversal.
    pub(crate) intake_priority: u8,
}

/// Number of priority points a player distributes across the yield axes.
const PRIORITY_TOTAL: u8 = 10;

/// Player directive: how to *balance* production across yields.
///
/// Priorities, not absolute targets. The entries sum to [`PRIORITY_TOTAL`]; the
/// allocator drives the *realized* yield distribution toward this priority
/// distribution. An axis's marginal value is its priority weighted against how
/// much it has already produced, so a high-priority axis draws proportionally
/// more population and balance emerges from diminishing weighted return. A zero
/// entry means "do not staff for this axis." A ratio is better UX than typing
/// output numbers and has no "above achievable maximum" awkwardness.
pub(crate) struct Goal {
    /// Priority weight per [`YieldKind`]; entries sum to [`PRIORITY_TOTAL`].
    pub(crate) priorities: [u8; YieldKind::COUNT],
}

impl Default for Goal {
    /// An even spread across all axes (the neutral "no preference" directive).
    fn default() -> Self {
        Self { priorities: [2; YieldKind::COUNT] }
    }
}

// ------------- Derived, cached (rebuilt on change) -------------

/// Index of a slot within a settlement's [`SlotTable`]. Local; counts are tiny,
/// so `u16` is ample even at metropolis scale.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotId(pub(crate) u16);

/// Index of a completion group within a [`SlotTable`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupId(pub(crate) u16);

/// The settlement's staffable slots, flattened from buildings and claimed-tile
/// improvements into one deterministic list for the allocator.
///
/// The nested record is ergonomic and is what adjacency bonuses need; the
/// allocator wants a flat, cache-friendly, stably-indexed list. This is the
/// bridge — derived from the record, holding both representations' value without
/// choosing between them.
///
/// Flattening order *is* the determinism tiebreak: districts in vector order,
/// then each district's complete buildings in vector order with their slots in
/// spec order, then claimed-tile improvements in canonical [`Coord`] order. Only
/// [`BuildState::Complete`] things contribute slots.
pub(crate) struct SlotTable {
    /// All staffable slots, in deterministic flatten order.
    pub(crate) slots: Vec<Slot>,
    /// One group per complete building that carries a completion bonus. The
    /// allocator's bundle move staffs all of a group's empty slots at once.
    pub(crate) groups: Vec<CompletionGroup>,
}

/// A single staffable position.
pub(crate) struct Slot {
    /// Yields produced per turn while staffed. A single slot may produce on
    /// several axes at once — resource improvements commonly yield a bulk good
    /// and gold together.
    pub(crate) produces: Yields,
    /// Completion group, if this slot belongs to a bonus-bearing building.
    /// Improvement slots and bonus-less buildings carry `None`.
    pub(crate) group: Option<GroupId>,
}

/// A building's completion bonus and the slots that must all be staffed for it.
///
/// Completion is scoped to a single building (never a district), so a group is
/// always a small fixed run regardless of era — the bundle move stays a small
/// constant at metropolis scale.
pub(crate) struct CompletionGroup {
    /// The building's slots: a contiguous run of indices into [`SlotTable::slots`].
    pub(crate) members: Range<u16>,
    /// Added to settlement yields when *every* member slot is staffed.
    pub(crate) bonus: Yields,
}

/// Which slots are staffed — the allocator's output, cached and read by the sweep.
pub(crate) struct Assignment {
    /// One flag per [`SlotId`]; `true` == staffed by one population. (A bitset
    /// once profiled; `Box<[bool]>` keeps the sketch dependency-free.)
    pub(crate) staffed: Box<[bool]>,
    /// Population consumed (count of set flags), cached for the upkeep math.
    pub(crate) used: u32,
}

/// Derived allocation state, rebuilt only on change — never per turn.
pub(crate) struct Plan {
    /// Flattened slots and completion groups for the current record.
    pub(crate) slots: SlotTable,
    /// The current staffing of those slots.
    pub(crate) assignment: Assignment,
}

// ------------- Entity decomposition -------------

/// Marker for settlement entities (cities and outposts alike).
#[derive(Component)]
pub(crate) struct Settlement;

/// The settlement's authoritative cross-machine identity (ADR-0024).
#[derive(Component)]
pub(crate) struct SettlementNetworkId(pub(crate) NetworkId);

/// Ownership and trade-network membership are **not** redefined here. A
/// settlement entity also carries [`crate::sim::empire::Owner`] (the one owner
/// key, `EmpireId`, shared with units and infrastructure — replacing the former
/// `SettlementOwner(PlayerId)`) and a [`crate::sim::empire::TradeNetworkId`]
/// stamped by the transit recompute. Both live in the leaf `empire` module so
/// that `economy` and `transit` share them without depending on each other.

/// The settlement's center tile.
#[derive(Component)]
pub(crate) struct SettlementCenter(pub(crate) Coord);

/// The authoritative economic record.
///
/// One coarse component, not scattered fields: allocation and the per-turn sweep
/// both touch the whole record, and the parallelism that matters is *across*
/// settlements (each entity independent — the island-decomposition surface), not
/// within one. The trade is that fine-grained ECS parallelism *within* a
/// settlement is foreclosed, which is acceptable because that work is serial
/// anyway.
///
/// In-flight construction bookkeeping is deliberately *absent* here — it lives in
/// the build subsystem. This component holds the result of construction, not its
/// process.
#[derive(Component)]
pub(crate) struct SettlementEconomy {
    /// Built districts and their buildings — the authoritative nested record.
    pub(crate) districts: Vec<District>,
    /// Derived index of the tiles this settlement claims, in claim order.
    ///
    /// **Not authoritative, not a duplicate.** Tile ownership and improvement
    /// identity are authoritative dense grid state (ADR-0023): "who owns coord X"
    /// and "what improvement sits on coord X" are coordinate-addressable queries
    /// the grid answers in O(1). This index runs the *other* direction —
    /// settlement → its tiles — so per-settlement work, above all the slot
    /// flattening walk over claimed improvements, enumerates "my tiles" without
    /// scanning the sparse, large map. Whether a claimed tile is improved is
    /// answered by reading the grid's improvement field at that coord, never by a
    /// field stored here, so the two cannot drift. Kept consistent by the single
    /// writer (the command processor), which mutates grid ownership and this list
    /// in the same step.
    pub(crate) claimed: Vec<Coord>,
    /// Center building handle. *This* determines tier (outpost vs. city) and
    /// district radius — the settlement is what its center makes it (§2.12), so
    /// promotion/demotion is swapping this handle, with no separate type flag.
    pub(crate) center: ContentHandle,
    /// Current population available to staff slots.
    pub(crate) population: u32,
    /// Network-local goods stock and capacity (ADR-0030).
    pub(crate) storage: Storage,
    /// The player's balance directive driving allocation.
    pub(crate) goal: Goal,
    /// Cached derivation. `None` == stale; rebuilt by [`allocate`] before the
    /// next sweep.
    pub(crate) plan: Option<Plan>,
}

// ------------- The two functions everything hangs on -------------

/// Allocates `population` across `slots` to best serve `goal`'s priorities.
///
/// Greedy with building-completion bundles. Candidate moves are *staff one slot*
/// and *staff a building's remaining empty slots as a bundle* (the bundle is what
/// captures a completion bonus, the one non-separable part of the problem). Each
/// candidate is scored by the priority-weighted marginal yield it adds, measured
/// against the running per-axis totals already assigned, so an axis's value
/// declines as it accumulates and the realized distribution converges on the
/// priority ratios. The best candidate commits, totals update, the field
/// re-scores, and the loop repeats until population is exhausted or no
/// positive-value candidate remains.
///
/// At Ancient scale this is flat greedy; the two accelerator seams are
/// deliberately *not* implemented here — a priority queue (when the AI goal
/// search makes this an inner-loop hot path) and a local-improvement pass (when
/// playtesting shows a settlement small enough for a human to beat the allocator
/// by a tempting margin). Both slot in behind this signature without touching a
/// caller.
///
/// Deterministic: scores compare as integers (ratios cross-multiplied, no
/// floating point), ties broken by ascending [`SlotId`]. Same inputs always
/// produce the same assignment — admissible in authoritative code and
/// reproducible for replay and tests, even though the layer-3 AI search that
/// calls this may itself be nondeterministic (it only emits commands that replay
/// through this path).
///
/// Pure: no `self`, no resources — trivially testable and reusable as the inner
/// loop of the AI goal search.
pub(crate) fn allocate(slots: &SlotTable, goal: &Goal, population: u32) -> Assignment {
    let _ = (slots, goal, population);
    todo!("greedy-with-bundles; see module docs and ADR-0028")
}

/// Sums realized yields for the current assignment — called once per settlement
/// in the turn sweep.
///
/// Sums `produces` over staffed slots (each slot may contribute on several
/// axes), adds each fully-staffed building's completion bonus, then applies
/// settlement-global `multipliers` in permille. Reads only; never mutates the
/// record or the assignment.
pub(crate) fn settlement_yields(
    slots: &SlotTable,
    assignment: &Assignment,
    multipliers: &YieldMultipliers,
) -> Yields {
    let _ = (slots, assignment, multipliers);
    todo!("staffed sum + completion bonuses + global multipliers")
}

// ------------- World-advance systems (resolution phase) -------------
//
// These run in the Resolving phase (`crate::sim::turn`), reading the locked
// start-of-turn snapshot, in this order: `produce_yields` → `resolve_supply`
// (which itself runs upkeep → construction → growth → surplus per component) →
// combat. They are economy systems, not transit ones: the trade-network partition
// is materialized as per-settlement `crate::sim::empire::TradeNetworkId` tags, so
// the reduce groups settlements by tag and needs no `transit` dependency.

/// Produces this turn's yields into authoritative stock — step 1 of the advance.
///
/// Per settlement, sums [`settlement_yields`] for its cached assignment and routes
/// by scope (ADR-0030): the network-local axes (food, materials) add to its own
/// [`Storage::stock`]; the empire-wide axes (gold, science, culture) add to the
/// owning empire's [`crate::sim::empire::Treasury`]. The only writes are those
/// increments. Deterministic over settlements in ascending settlement key.
pub(crate) fn produce_yields() {
    todo!("settlement_yields per settlement; network-local -> Storage, empire-wide -> Treasury")
}

/// Resolves one turn of each trade network's economy — steps 2–3 of the advance:
/// pool, consume demand-first, then distribute the surplus.
///
/// Groups settlements by their [`crate::sim::empire::TradeNetworkId`] (the
/// materialized partition) and, per component, treats member stock as one **pooled
/// budget** — within-advance scratch, never stored, consistent with the network
/// being derived. Demand is met from the pool in a fixed order:
///
/// 1. **Upkeep** — unit food upkeep (ADR-0031) drawn from the pool; a shortfall is
///    starvation (ADR-0031 / `cities.md` §2.6), not a stall. *Which* units/pop are
///    cut on an unmet shortfall lives in those docs, not here.
/// 2. **Construction** — `jobs::advance_construction` (city build-slots) and
///    `worker::advance_worker_jobs` (workers) draw materials from the pool. When
///    materials are short the queue's existing priority order decides funding
///    (top-first; lower jobs stall with progress intact, since jobs are
///    site-owned); across settlements sharing the component, ascending settlement
///    key. That cross-settlement order is a deterministic default, not a balance
///    statement — revisitable tuning, not structure.
/// 3. **Growth** — [`apply_growth`] draws food from the remaining pool.
///
/// **Food is upkeep before growth**: a shortfall halts expansion before it starves
/// what exists, so a standing army drags on growth — intended pressure. Food and
/// materials never compete (separate axes), so these are the only orderings that
/// matter.
///
/// Then [`distribute_surplus`] returns the pooled remainder to member storages.
/// Deterministic over components in ascending `TradeNetworkId`.
pub(crate) fn resolve_supply() {
    todo!("pool component (scratch); upkeep -> construction -> growth; then distribute_surplus")
}

/// Advances one turn of population growth from available food — the growth step of
/// [`resolve_supply`], factored out pure for testability.
///
/// Consumes food from the component pool to advance the settlement's growth per
/// `cities.md` §2.6 (the growth-loop math lives there, not duplicated here), and
/// returns the food actually consumed so the caller deducts it from the pool. A
/// food deficit is the §2.6 shrink path. Pure and deterministic.
pub(crate) fn apply_growth(economy: &mut SettlementEconomy, food_available: i32) -> i32 {
    let _ = (economy, food_available);
    todo!("advance growth per cities.md §2.6; return food consumed")
}

/// Distributes a component's post-demand surplus across its members by intake
/// priority — the final step of [`resolve_supply`], factored out pure for
/// testability.
///
/// Fills storages in **descending `intake_priority`** (a `0` never receives and was
/// drained first; ties broken by ascending settlement key, so `members` are passed
/// in that order), each clamped to `Storage::capacity`; whatever cannot be stored
/// anywhere is lost (ADR-0030). A sort over members, not a graph traversal.
/// `members` is one trade network's storages; `surplus` is the pooled remainder
/// after demand.
pub(crate) fn distribute_surplus(members: &mut [Storage], surplus: Yields) {
    let _ = (members, surplus);
    todo!("descending intake_priority, clamp to capacity, overflow lost")
}
