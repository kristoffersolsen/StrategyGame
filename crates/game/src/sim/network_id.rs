//! Deterministic cross-machine entity identifiers.
//!
//! [`NetworkId`] is the authoritative identity for units, cities, and other
//! game actors. Unlike Bevy [`Entity`] indices, `NetworkId`s are assigned in
//! command-processing order and are therefore identical across all machines in
//! a multiplayer game. They are the cross-tile reference type stored in
//! [`super::world_model::TileState`].
//!
//! The derived [`Entity`] lookup is non-authoritative and is never serialised
//! or synced. See ADR-0024.

use std::collections::HashMap;

use bevy::prelude::{Entity, Resource};

/// A stable, deterministic identifier for a game actor (unit, city, etc.).
///
/// Assigned by [`NetworkIdAllocator`] in command-processing order.
/// Identical across all machines; never derived from a Bevy [`Entity`] index.
/// Only [`NetworkId`] crosses save files and the network — [`Entity`] is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NetworkId(u64);

/// Monotonic counter that allocates [`NetworkId`]s in command-processing order.
///
/// Must only be advanced from within the command processor so that allocation
/// order is deterministic across machines. Never randomised, never derived from
/// an [`Entity`] index.
#[derive(Resource, Default)]
pub(crate) struct NetworkIdAllocator {
    next: u64,
}

impl NetworkIdAllocator {
    /// Allocates the next [`NetworkId`] and advances the counter.
    ///
    /// Called exclusively from the command processor (Stage 5+) to ensure
    /// allocation order is deterministic. Not called in non-command code.
    ///
    /// # Panics
    ///
    /// Panics if the counter overflows `u64::MAX`, which is not a realistic
    /// game condition.
    pub(crate) fn next(&mut self) -> NetworkId {
        let id = NetworkId(self.next);
        self.next = self
            .next
            .checked_add(1)
            .expect("NetworkId counter must not overflow");
        id
    }

    /// Returns the current counter value without advancing it.
    ///
    /// Intended for determinism tests that need to snapshot and compare
    /// allocator state after a known command sequence.
    #[cfg(test)]
    pub(crate) fn peek(&self) -> u64 {
        self.next
    }
}

/// Non-authoritative [`NetworkId`] → [`Entity`] lookup cache.
///
/// Rebuilt from authoritative state after command processing; never
/// serialised or synced. Use [`NetworkId`] as the canonical reference
/// in tile state and save files — this cache is a convenience only.
/// See ADR-0024.
#[derive(Resource, Default)]
pub(crate) struct EntityLookup {
    map: HashMap<NetworkId, Entity>,
}

impl EntityLookup {
    /// Inserts or updates the entity binding for a [`NetworkId`].
    pub(crate) fn insert(&mut self, id: NetworkId, entity: Entity) {
        self.map.insert(id, entity);
    }

    /// Removes the entity binding for a [`NetworkId`].
    // Stage 7+: called when an entity is despawned (e.g. settler consumed on city founding).
    #[allow(dead_code)]
    pub(crate) fn remove(&mut self, id: NetworkId) {
        self.map.remove(&id);
    }

    /// Returns the entity bound to `id`, if any.
    pub(crate) fn get(&self, id: NetworkId) -> Option<Entity> {
        self.map.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_is_sequential_from_zero() {
        let mut alloc = NetworkIdAllocator::default();
        let ids: Vec<NetworkId> = (0..5).map(|_| alloc.next()).collect();
        assert_eq!(
            ids,
            vec![
                NetworkId(0),
                NetworkId(1),
                NetworkId(2),
                NetworkId(3),
                NetworkId(4)
            ]
        );
    }

    #[test]
    fn identical_starting_states_produce_identical_sequences() {
        let mut alloc_a = NetworkIdAllocator::default();
        let mut alloc_b = NetworkIdAllocator::default();

        let seq_a: Vec<NetworkId> = (0..10).map(|_| alloc_a.next()).collect();
        let seq_b: Vec<NetworkId> = (0..10).map(|_| alloc_b.next()).collect();

        assert_eq!(
            seq_a, seq_b,
            "identical starting states must produce identical id sequences"
        );
    }

    #[test]
    fn peek_does_not_advance_counter() {
        let mut alloc = NetworkIdAllocator::default();
        let _first = alloc.next();
        assert_eq!(alloc.peek(), 1);
        assert_eq!(alloc.peek(), 1, "peek must be idempotent");
    }

    #[test]
    fn entity_lookup_insert_get_remove() {
        let mut lookup = EntityLookup::default();
        let id = NetworkId(42);
        let entity = Entity::from_bits(7_u64);

        assert_eq!(lookup.get(id), None);
        lookup.insert(id, entity);
        assert_eq!(lookup.get(id), Some(entity));
        lookup.remove(id);
        assert_eq!(lookup.get(id), None);
    }
}
