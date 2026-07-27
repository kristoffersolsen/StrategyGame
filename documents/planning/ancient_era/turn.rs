//! Turn loop: the two phases of a turn and the deterministic world-advance.
//!
//! Intended path `game::sim::turn`. A turn is one authoritative state touched in
//! two phases — there is no second "next-turn" state, only two modes of mutation:
//!
//! - [`TurnPhase::Interactive`] — controllers (human or AI, one path) emit
//!   *commands* that complete **live**, in server-assigned sequence. Nothing
//!   advances: no yields accrue, no growth, no construction progress, no combat.
//!   Connectivity commands fire [`crate::sim::transit::recompute_trade_networks`]
//!   and record/goal/population changes re-run `economy::allocate` here, so the
//!   player sees the consequence of each command as it lands.
//! - [`TurnPhase::Resolving`] — human input is locked; AI empires emit their
//!   commands (same live, sequenced path, so they act on the turn's human moves);
//!   then the world advances one tick (the ordered pipeline below).
//!
//! # Determinism invariant (pillar 0.4): the canonical command log
//!
//! The server assigns each command a [`CommandSeq`] **as it arrives**, and that
//! sequenced log is the *sole order of record*. Clients and replays re-apply the
//! log; they never re-derive order from raw inputs (that would desync on latency).
//! Arrival order — not a tie-break rule — adjudicates interactive contention: the
//! second empire to settle a tile finds it already taken, so its command is simply
//! invalid against current state. An empire's own commands keep their relative
//! order inside the global sequence (per-empire FIFO falls out for free). Tests
//! issue from one machine, so order is controlled and reproducible.
//!
//! The three setup modes (simultaneous / sequential / mixed) are **input-gating
//! policies on top of this one path** — who may submit when — not three loops. The
//! apply path, the log, and the advance below are identical across them.
//!
//! # The simultaneity seam is in combat, not here
//!
//! Interactive contention is handled by arrival order. The only place that still
//! needs simultaneity rules is [`resolve_combat`], which runs in the advance (step
//! 4): armies clash against the locked snapshot, not the instant a move arrives.
//! That seam is the open combat-model decision; its body is `todo!()`.
//!
//! # World-advance order (reads the locked start-of-turn snapshot)
//!
//! 1. **Produce** — `economy::produce_yields`: network-local axes to `Storage`,
//!    empire-wide axes to `Treasury`.
//! 2. **Consume (demand-first, per component)** — pool the trade network, then
//!    satisfy upkeep, construction (`jobs::advance_construction`,
//!    `worker::advance_worker_jobs`), and growth from the pool.
//! 3. **Distribute surplus** — `economy::distribute_supply`: fill member storages
//!    by descending `intake_priority`; overflow past capacity is lost.
//! 4. **Combat** — [`resolve_combat`] (the simultaneity seam).
//! 5. **Tick** — advance the counter, broadcast, reopen [`TurnPhase::Interactive`].
//!
//! Combat is **last** so the advance never recomputes the partition mid-pipeline:
//! connectivity is current from the interactive recompute, and combat's ownership
//! changes fire an event-recompute for the *next* turn. The S2 saturation snapshot,
//! when it lands, is taken before step 1 (pre-production stock) — the acyclic cut.
//!
//! Singleplayer is the degenerate single-command-source case of this same loop, so
//! the integrated-server path (Stages 4–6) reshapes nothing.

use bevy::prelude::Resource;

/// The phase a turn is in. Drives a Bevy `States` transition at wiring time; kept
/// a plain enum in the sketch.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnPhase {
    /// Controllers emit commands; they complete live; the world is frozen.
    Interactive,
    /// Input locked; AI emits; the world advances one tick.
    Resolving,
}

/// Monotonic, server-assigned command sequence number — the sole order of record.
///
/// Stamped on arrival and never reconstructed from raw inputs. The ordering this
/// imposes is what makes the live-arrival model replay-deterministic under server
/// authority (see module docs).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CommandSeq(pub(crate) u64);

/// Current turn number and phase. The `next` source the sequencer hands out.
#[derive(Resource)]
pub(crate) struct TurnState {
    /// Turn counter, advanced at tick.
    pub(crate) turn: u32,
    /// Which phase the loop is in.
    pub(crate) phase: TurnPhase,
    /// Next [`CommandSeq`] to assign. Monotonic for the whole game.
    pub(crate) next_seq: u64,
}

// ------------- Phase transitions -------------

/// Locks human input and hands the turn to the AI + advance — the
/// [`Interactive`](TurnPhase::Interactive) → [`Resolving`](TurnPhase::Resolving)
/// edge. Stops accepting human commands; AI commands still flow through the same
/// sequenced path during `Resolving`.
pub(crate) fn lock_inputs() {
    todo!("transition to Resolving; stop admitting human commands")
}

/// Reopens input after the advance — the `Resolving` → `Interactive` edge — having
/// advanced the counter and broadcast the new state.
pub(crate) fn reopen_inputs() {
    todo!("advance TurnState::turn, broadcast, transition to Interactive")
}

// ------------- World-advance: the combat seam -------------

/// Resolves combat against the locked snapshot — **the simultaneity seam**.
///
/// Runs as step 4 of the advance, after the economy has produced, consumed, and
/// distributed. Whether contending empires resolve by instantaneous rule
/// (simultaneous) or by initiative within a conflict cluster (sequential / mixed)
/// is the open combat-model decision; this is where that policy lives, and it is
/// the prerequisite flagged before any combat threading. Captures and unit
/// destruction here fire an event-recompute of the affected empires' partitions
/// for the next turn (combat-last → no mid-advance relabel).
///
/// Moves to a dedicated `combat` module once that exists; parked here as the seam
/// the loop schedules.
pub(crate) fn resolve_combat() {
    todo!("simultaneity policy: rule-adjudicated (now) vs initiative clusters (later)")
}
