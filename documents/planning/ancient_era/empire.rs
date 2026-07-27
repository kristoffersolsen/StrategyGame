//! Empire: the authoritative owner of settlements, units, and infrastructure.
//!
//! Intended path `game::sim::empire`. An **empire** is the only thing the
//! simulation treats as an owner. It *embodies* a civilization (the composable,
//! worldview-bearing identity, ADR-0022) and is *run by* a controller (human or
//! AI) — but neither identity nor control is an owner: they are attributes of an
//! empire, not separate owners. There is deliberately **no `Player`** in
//! authoritative state. The human-client ↔ empire mapping is a session/transport
//! concern that does not replay; the command stream does.
//!
//! # Why this is a leaf module
//!
//! It defines the owner key ([`EmpireId`]), the owner tag ([`Owner`]), and the
//! trade-network partition label ([`TradeNetworkId`]) while importing nothing
//! from `economy` or `transit`. That is deliberate: `transit` *produces* the
//! partition label (it flood-fills road tiles) and `economy` *consumes* it (for
//! pooling and distribution), so if either module owned the type the other would
//! have to depend on it. Parking the shared keys in a leaf lets both depend on
//! `empire` and not on each other — no cycle.
//!
//! `RoadPool` is likewise attached to the empire *entity* as its own component
//! (it lives in `transit`), not nested into a struct here, so `empire` never has
//! to import `transit`.

use bevy::prelude::Component;

/// Dense owner / iteration handle.
///
/// The roster is fixed at game start — no mid-game joins — so a `u8` index is
/// stable for the whole game and gives deterministic per-empire iteration
/// (pillar 0.4) without an allocator. An empire may switch [`ControllerMode`],
/// but its id never changes. Carried by the empire entity; wrapped in [`Owner`]
/// on everything the empire owns.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EmpireId(pub(crate) u8);

/// Marker for the empire entity.
#[derive(Component)]
pub(crate) struct Empire;

/// Who drives an empire's decisions.
///
/// A *mode*, not a different owner: switching it (e.g. a human dropping to AI
/// control) is a command and changes nothing about ownership, ids, or replay.
#[derive(Component)]
pub(crate) enum ControllerMode {
    /// Driven by a human client this session.
    Human,
    /// Driven by the AI strategist (systems_overview S6).
    Ai,
}

/// The owning empire of a settlement, unit, or piece of infrastructure.
///
/// The one owner key in the simulation — it replaces the former
/// `SettlementOwner(PlayerId)` and is shared by units and infrastructure, not
/// just settlements, so "who owns this" is one question with one answer type.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Owner(pub(crate) EmpireId);

/// An empire's pool of the empire-wide goods — gold, science, culture (ADR-0030):
/// the goods that do **not** route through the trade-network partition.
///
/// Held on the empire entity, not the settlement (settlements hold only the
/// network-local goods, in `economy::Storage`). Gold is a true accumulating
/// stock. Whether science and culture *accumulate* here across turns or are
/// drained each turn into progress trackers (research, civics) is an open
/// sub-decision; either way this is their empire-scoped collection point. Kept as
/// three explicit axes rather than reusing `economy::Yields` so this module stays
/// a leaf and the "only some axes are meaningful" caveat never arises.
#[derive(Component, Default)]
pub(crate) struct Treasury {
    /// Accumulating currency.
    pub(crate) gold: i32,
    /// Research output collected empire-wide (accumulate-vs-drain: open).
    pub(crate) science: i32,
    /// Cultural output collected empire-wide (accumulate-vs-drain: open).
    pub(crate) culture: i32,
}

/// Within-turn label of a **trade network** — one road-connected component of a
/// single empire's settlements (the supply partition, ADR-0030).
///
/// Stamped on each settlement by
/// [`recompute_trade_networks`](crate::sim::transit::recompute_trade_networks)
/// and read by the economy for pooling and surplus distribution. Two properties
/// the name and type are chosen to protect:
///
/// - **Not stable across turns.** It is a recompute-local partition label, not an
///   identity. Pooling groups settlements by *equal label this turn* and sums
///   their `Storage`; the value is never persisted or referenced across a
///   recompute, so merge and severance are the same operation (relabel).
/// - **Not an entity handle.** Distinct from the cross-machine
///   [`NetworkId`](crate::sim::network_id::NetworkId) (ADR-0024) — the "network"
///   collision is real and the separate name is deliberate. Nothing in the supply
///   path takes a `NetworkId`.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TradeNetworkId(pub(crate) u16);
