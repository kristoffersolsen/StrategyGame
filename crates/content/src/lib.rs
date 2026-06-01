//! Shared content-registry types for the game and worldgen crates.
//!
//! Content kinds — biomes, resources, units, and so on — are identified
//! by stable string IDs that cross save files and the network, and
//! represented at runtime as dense [`Handle`] indices into a [`Registry`].
//!
//! # Design
//!
//! String IDs are the authoritative identity. Handles are derived at
//! load time by sorting all registered IDs alphabetically and assigning
//! them consecutive dense indices. Handles are **never serialised**; only
//! string IDs are.
//!
//! This mirrors the `NetworkId`-vs-`Entity` split: a stable authority, a
//! derived runtime index, and a hard rule that only the former is
//! persisted or synced. See ADR-0022.
//!
//! # Hot-path discipline
//!
//! String resolution happens **once at load boundaries**. After that,
//! all per-tile or per-turn logic works exclusively through handles and
//! dense property arrays (e.g. `is_forest: Vec<bool>`).

/// A dense runtime index into a [`Registry`].
///
/// Handles are derived from string IDs by alphabetical ordering. The
/// same set of string IDs always produces the same handle assignments,
/// making generation fully deterministic.
///
/// Handles are `Copy` and cheap to compare; they should be used for
/// all hot-path logic. String IDs are only needed at load/save
/// boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handle(u32);

impl Handle {
    /// Returns the dense zero-based index for this handle.
    ///
    /// Suitable for indexing into parallel property arrays
    /// (e.g. `registry.is_forest[handle.index()]`).
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Constructs a handle from a raw `u32` index.
    ///
    /// This is a low-level constructor for use during table construction
    /// where a placeholder handle is needed before resolution. Prefer
    /// [`Registry::resolve`] at normal call sites.
    #[must_use]
    pub fn from_raw(index: u32) -> Self {
        Self(index)
    }
}

/// A sorted string-ID → [`Handle`] mapping.
///
/// The registry assigns dense indices alphabetically: the
/// lexicographically smallest ID gets handle 0, the next gets handle 1,
/// and so on. This guarantees that a given set of IDs always maps to the
/// same handles regardless of the insertion order they were listed in.
///
/// # Construction
///
/// Use [`Registry::build`] to construct a registry. Duplicate IDs are
/// silently deduplicated.
///
/// # Querying
///
/// - [`Self::resolve`] — string ID → `Option<Handle>`
/// - [`Self::id_of`] — handle → `&str`
/// - [`Self::len`] / [`Self::is_empty`] — entry count
/// - [`Self::handles`] — iterate all handles in index order
/// - [`Self::iter`] — iterate `(handle, id)` pairs in index order
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    /// Sorted string IDs. Index in this vec equals `handle.index()`.
    ids: Vec<String>,
}

impl Registry {
    /// Builds a registry from a collection of string IDs.
    ///
    /// IDs are sorted alphabetically and deduplicated. The resulting
    /// handle assignment is deterministic for a given input set,
    /// regardless of the order the IDs are supplied in.
    pub fn build(ids: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut sorted: Vec<String> = ids.into_iter().map(|s| s.as_ref().to_owned()).collect();
        sorted.sort_unstable();
        sorted.dedup();
        Self { ids: sorted }
    }

    /// Resolves a string ID to a handle.
    ///
    /// Returns `None` if the ID is not registered.
    ///
    /// # Panics
    ///
    /// Panics if the registry contains more than `u32::MAX` entries, which
    /// is not a realistic condition for any game content registry.
    #[must_use]
    pub fn resolve(&self, id: &str) -> Option<Handle> {
        self.ids
            .binary_search_by_key(&id, String::as_str)
            .ok()
            .map(|idx| Handle(u32::try_from(idx).expect("registry index fits in u32")))
    }

    /// Returns the string ID for a handle.
    ///
    /// # Panics
    ///
    /// Panics if `handle` was not produced by this registry. Since
    /// handles can only be produced by `resolve`, this can only happen
    /// if handles from different registries are mixed.
    #[must_use]
    pub fn id_of(&self, handle: Handle) -> &str {
        &self.ids[handle.index()]
    }

    /// Returns the number of registered IDs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns `true` if no IDs are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Returns an iterator over all handles in ascending index order.
    ///
    /// # Panics
    ///
    /// Panics if the registry contains more than `u32::MAX` entries, which
    /// is not a realistic condition for any game content registry.
    pub fn handles(&self) -> impl Iterator<Item = Handle> {
        (0..self.ids.len()).map(|i| Handle(u32::try_from(i).expect("registry index fits in u32")))
    }

    /// Returns an iterator over `(handle, id)` pairs in ascending
    /// index order.
    ///
    /// # Panics
    ///
    /// Panics if the registry contains more than `u32::MAX` entries, which
    /// is not a realistic condition for any game content registry.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &str)> {
        self.ids.iter().enumerate().map(|(i, id)| {
            (
                Handle(u32::try_from(i).expect("registry index fits in u32")),
                id.as_str(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_sorts_alphabetically() {
        let r = Registry::build(["tundra", "desert", "grassland"]);
        let ids: Vec<&str> = r.iter().map(|(_, id)| id).collect();
        assert_eq!(ids, ["desert", "grassland", "tundra"]);
    }

    #[test]
    fn build_deduplicates() {
        let r = Registry::build(["iron", "iron", "gold"]);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn resolve_round_trips() {
        let r = Registry::build(["tundra", "desert", "grassland"]);
        for (handle, id) in r.iter() {
            let resolved = r.resolve(id).expect("registered id must resolve");
            assert_eq!(resolved, handle);
            assert_eq!(r.id_of(resolved), id);
        }
    }

    #[test]
    fn unknown_id_returns_none() {
        let r = Registry::build(["iron"]);
        assert_eq!(r.resolve("gold"), None);
    }

    #[test]
    fn deterministic_ordering_independent_of_insertion_order() {
        let r1 = Registry::build(["tundra", "grassland", "desert"]);
        let r2 = Registry::build(["desert", "tundra", "grassland"]);
        // Same set in different order must produce identical handle assignments.
        assert_eq!(r1.resolve("desert"), r2.resolve("desert"));
        assert_eq!(r1.resolve("grassland"), r2.resolve("grassland"));
        assert_eq!(r1.resolve("tundra"), r2.resolve("tundra"));
    }

    #[test]
    fn handles_are_dense_zero_based() {
        let r = Registry::build(["c", "a", "b"]);
        let handles: Vec<usize> = r.handles().map(Handle::index).collect();
        assert_eq!(handles, [0, 1, 2]);
    }

    #[test]
    fn empty_registry() {
        let empty: &[&str] = &[];
        let r = Registry::build(empty.iter().copied());
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert_eq!(r.resolve("anything"), None);
    }
}
