//! RNG threading for the world-generation pipeline.
//!
//! Implements the strategy documented in ADR-0001: a single
//! [`MasterRng`] is constructed from the master seed, and each
//! pipeline stage receives its own independent [`StageRng`] derived
//! via [`MasterRng::fork`].
//!
//! # Why this matters
//!
//! Without stage-independent sub-RNGs, tuning a single parameter in
//! (say) the climate stage would shift every random draw in every
//! downstream stage, making iterative tuning impractical. With this
//! design, each stage's RNG is a deterministic function of only
//! `(master_seed, StageId)` — completely independent of how many
//! numbers any other stage consumed.
//!
//! # Usage
//!
//! ```
//! use worldgen::rng::{MasterRng, StageId};
//! use rand_core::Rng;
//!
//! let master = MasterRng::new(0xDEAD_BEEF);
//!
//! // Each fork is independent: the climate RNG is unaffected by how
//! // many numbers the tectonics RNG consumed, and vice versa.
//! let mut tectonics_rng = master.fork(StageId::Tectonics);
//! let mut climate_rng   = master.fork(StageId::Climate);
//!
//! // Forking is idempotent: the same stage always yields the same
//! // first draw.
//! let a = master.fork(StageId::Climate).next_u64();
//! let b = master.fork(StageId::Climate).next_u64();
//! assert_eq!(a, b);
//! # let _ = tectonics_rng.next_u64();
//! # let _ = climate_rng.next_u64();
//! ```
//!
//! # Rules enforced by lint and review (ADR-0009)
//!
//! - `rand::thread_rng` and `rand::rng` are forbidden workspace-wide.
//! - `HashMap` iteration must not affect generation output; use
//!   `BTreeMap` or sort before iterating.
//! - No parallel reductions in inner generation loops.

use det_rng::{DetRng, derive_seed};
use rand_core::{Infallible, Rng, TryRng};

/// The master RNG for a single world-generation run.
///
/// Constructed once from the [`crate::WorldParams`] seed and passed
/// into the pipeline entry point. Each stage receives its own
/// independent [`StageRng`] via [`MasterRng::fork`].
///
/// `MasterRng` itself does not implement [`Rng`] — it is not a
/// source of random numbers, only a factory for stage-specific
/// sources. This distinction is intentional: it prevents accidentally
/// drawing numbers from the master directly, which would make the
/// output sensitive to call order.
#[derive(Debug, Clone)]
pub struct MasterRng {
    seed: u64,
}

impl MasterRng {
    /// Constructs a new [`MasterRng`] from a master seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Derives an independent [`StageRng`] for the given stage.
    ///
    /// The returned RNG's seed is a deterministic function of
    /// `(master_seed, stage_id)` only — it does not depend on the
    /// order in which `fork` is called, or on how many numbers any
    /// other stage has consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use worldgen::rng::{MasterRng, StageId};
    /// use rand_core::Rng;
    ///
    /// let master = MasterRng::new(42);
    ///
    /// // Same stage always produces the same first draw.
    /// let first_a = master.fork(StageId::Tectonics).next_u64();
    /// let first_b = master.fork(StageId::Tectonics).next_u64();
    /// assert_eq!(first_a, first_b);
    ///
    /// // Different stages produce different streams.
    /// let tec = master.fork(StageId::Tectonics).next_u64();
    /// let cli = master.fork(StageId::Climate).next_u64();
    /// assert_ne!(tec, cli);
    /// ```
    #[must_use]
    pub fn fork(&self, stage: StageId) -> StageRng {
        StageRng {
            inner: DetRng::from_seed(derive_seed(self.seed, stage.salt())),
        }
    }
}

/// An independent RNG stream for a single pipeline stage.
///
/// Obtained via [`MasterRng::fork`]. Implements [`Rng`] so it
/// works directly with any `rand` extension method (`gen_range`,
/// `sample`, `shuffle`, etc.) on any type that accepts `R: Rng`.
///
/// `StageRng` deliberately cannot itself fork — sub-stage splits
/// within a single stage should be achieved by drawing seed material
/// from this RNG and constructing a new [`DetRng`] from
/// [`det_rng::DetRng::from_seed`]. This keeps the threading topology
/// shallow and easy to reason about.
#[derive(Debug, Clone)]
pub struct StageRng {
    inner: DetRng,
}

impl TryRng for StageRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(self.inner.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(self.inner.next_u64())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        self.inner.fill_bytes(dst);
        Ok(())
    }
}

/// Identifies a pipeline stage for sub-seed derivation.
///
/// Each variant carries an explicit `u64` salt constant (see
/// [`StageId::salt`]) rather than deriving the value from the enum
/// discriminant. This means reordering variants — whether for
/// readability or to insert a new stage — cannot silently shift any
/// sub-seed.
///
/// The constants are large primes chosen to be well-distributed in
/// their bit patterns. They have no significance beyond being
/// distinct and stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageId {
    /// Tectonic plate generation and base elevation (design §1.9).
    Tectonics,
    /// Temperature, moisture, and advection (design §1.10).
    Climate,
    /// Flow accumulation, rivers, lakes, canyons (design §1.11).
    Hydrology,
    /// Biome classification and feature placement (design §1.12).
    Biomes,
    /// Luxury and strategic resource placement (design §1.13).
    Resources,
    /// Starting position selection and adequacy checking (design §1.14).
    StartingPositions,
}

impl StageId {
    /// Returns the stable salt constant used when deriving this
    /// stage's sub-seed.
    ///
    /// Values are large primes with no further significance. They are
    /// hardcoded here — not derived from discriminants — so that
    /// reordering the enum cannot shift any sub-seed.
    #[must_use]
    pub const fn salt(self) -> u64 {
        match self {
            Self::Tectonics => 0x9E37_79B9_7F4A_7C55,
            Self::Climate => 0x6C62_272E_07BB_0142,
            Self::Hydrology => 0xB492_B66F_BE98_F273,
            Self::Biomes => 0x9AE1_6A3B_2F90_404F,
            Self::Resources => 0xCBF2_9CE4_8422_2325,
            Self::StartingPositions => 0x517C_C1B7_2722_0A95,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::Rng;

    // ------------- MasterRng::fork -------------

    #[test]
    fn fork_is_deterministic() {
        let master = MasterRng::new(0xDEAD_BEEF);
        let a = master.fork(StageId::Tectonics).next_u64();
        let b = master.fork(StageId::Tectonics).next_u64();
        assert_eq!(a, b);
    }

    #[test]
    fn fork_is_order_independent() {
        // Forking Climate before Tectonics must yield the same
        // streams as forking Tectonics before Climate.
        let master = MasterRng::new(99);

        let tec_first = master.fork(StageId::Tectonics).next_u64();
        let cli_first = master.fork(StageId::Climate).next_u64();

        // (reverse order)
        let cli_second = master.fork(StageId::Climate).next_u64();
        let tec_second = master.fork(StageId::Tectonics).next_u64();

        assert_eq!(tec_first, tec_second);
        assert_eq!(cli_first, cli_second);
    }

    #[test]
    fn different_stages_produce_different_streams() {
        let master = MasterRng::new(1);
        let draws: Vec<u64> = [
            StageId::Tectonics,
            StageId::Climate,
            StageId::Hydrology,
            StageId::Biomes,
            StageId::Resources,
            StageId::StartingPositions,
        ]
        .iter()
        .map(|&s| master.fork(s).next_u64())
        .collect();

        // All first draws must be distinct.
        let unique: std::collections::BTreeSet<u64> = draws.iter().copied().collect();
        assert_eq!(unique.len(), draws.len(), "two stages share a first draw");
    }

    #[test]
    fn different_seeds_produce_different_streams() {
        let a = MasterRng::new(1).fork(StageId::Tectonics).next_u64();
        let b = MasterRng::new(2).fork(StageId::Tectonics).next_u64();
        assert_ne!(a, b);
    }

    // ------------- StageId salts -------------

    #[test]
    fn all_salts_are_distinct() {
        let salts: Vec<u64> = [
            StageId::Tectonics,
            StageId::Climate,
            StageId::Hydrology,
            StageId::Biomes,
            StageId::Resources,
            StageId::StartingPositions,
        ]
        .iter()
        .map(|s| s.salt())
        .collect();

        let unique: std::collections::BTreeSet<u64> = salts.iter().copied().collect();
        assert_eq!(
            unique.len(),
            salts.len(),
            "two StageId variants share a salt"
        );
    }
}
