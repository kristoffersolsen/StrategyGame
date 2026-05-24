//! Deterministic-RNG primitives shared by `worldgen` and `game`.
//!
//! This crate exposes the load-bearing pieces of the project's
//! determinism strategy (design doc §0.4, ADR-0009):
//!
//! - [`DetRng`] — the canonical deterministic RNG type. Wraps
//!   [`rand_chacha::ChaCha8Rng`] so the rest of the workspace
//!   references a single named type with a documented stability
//!   guarantee.
//! - [`derive_seed`] — combines a master seed with a domain-specific
//!   salt to produce an independent sub-seed.
//! - [`splitmix64`] — the underlying bit mixer. Exposed because it is
//!   occasionally useful directly.
//!
//! # Stability guarantee
//!
//! For a given `(master_seed, salt)`, [`derive_seed`] returns the same
//! `u64` on every platform and across every supported Rust toolchain.
//! For a given seed, [`DetRng`] produces the same byte stream on every
//! platform.
//!
//! Both guarantees are unconditional: this crate does not use floating
//! point, hash maps, or any other source of nondeterminism. Tests in
//! the determinism scaffold pin the output against known reference
//! values; changes to either guarantee will be flagged by CI.
//!
//! # Why a separate crate
//!
//! Both world generation and the runtime game loop need deterministic
//! RNG (design §0.4), but they need *different* higher-level APIs:
//! `worldgen` derives sub-RNGs per generation stage, while the runtime
//! will derive sub-RNGs per game system, per player, and must support
//! save-load round-trips. This crate provides only the shared
//! primitive; each consumer builds its own API on top.

#![forbid(unsafe_code)]

use rand_chacha::ChaCha8Rng;
use rand_core::{Infallible, Rng, SeedableRng, TryRng};

/// `SplitMix64` bit mixer.
///
/// Maps a `u64` to a `u64` such that small changes in the input
/// produce well-distributed changes in the output. Used as the
/// building block for [`derive_seed`].
///
/// This is the same algorithm Java's `SplittableRandom` uses to derive
/// independent sub-streams. It is not cryptographic — do not use it
/// where adversarial inputs matter — but it is more than sufficient
/// for deriving uncorrelated seeds inside a trust boundary.
///
/// # Examples
///
/// ```
/// use det_rng::splitmix64;
///
/// // Deterministic: same input always yields the same output.
/// assert_eq!(splitmix64(42), splitmix64(42));
///
/// // Small input changes produce uncorrelated outputs.
/// assert_ne!(splitmix64(42), splitmix64(43));
/// ```
#[must_use]
pub const fn splitmix64(mut x: u64) -> u64 {
    // Constants from Sebastiano Vigna's reference implementation:
    // https://prng.di.unimi.it/splitmix64.c
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Derives a sub-seed from a master seed and a domain-specific salt.
///
/// The returned value is a deterministic function of both inputs and
/// is suitable for seeding an independent [`DetRng`]. Each `(master,
/// salt)` pair produces an effectively uncorrelated sub-seed, so a
/// caller may use the same `master` with many different `salt` values
/// to obtain independent random streams.
///
/// The mixing combines the two inputs before passing them through
/// [`splitmix64`], so the output is not merely `splitmix64(master ^
/// salt)` — that would collide whenever two `(master, salt)` pairs
/// XOR to the same value.
///
/// # Examples
///
/// ```
/// use det_rng::derive_seed;
///
/// // Determinism: same inputs → same output.
/// assert_eq!(derive_seed(0xDEADBEEF, 1), derive_seed(0xDEADBEEF, 1));
///
/// // Independence: different salts → different sub-seeds.
/// assert_ne!(derive_seed(0xDEADBEEF, 1), derive_seed(0xDEADBEEF, 2));
///
/// // Independence: different masters → different sub-seeds.
/// assert_ne!(derive_seed(1, 42), derive_seed(2, 42));
/// ```
#[must_use]
pub const fn derive_seed(master: u64, salt: u64) -> u64 {
    // Mix master and salt through two splitmix64 rounds so the
    // contribution of each input is dispersed before they meet. This
    // avoids the trivial collision `derive_seed(a, b) ==
    // derive_seed(a ^ x, b ^ x)`.
    let m = splitmix64(master);
    let s = splitmix64(salt);
    splitmix64(m ^ s.wrapping_add(0x9E37_79B9_7F4A_7C15))
}

/// The project's canonical deterministic RNG.
///
/// A newtype wrapper around [`rand_chacha::ChaCha8Rng`]. `ChaCha8` is
/// chosen because its output for a given seed is documented as stable
/// by the `rand_chacha` crate, which is required for the project's
/// determinism property to hold across releases.
///
/// `DetRng` deliberately exposes only the [`Rng`] interface plus
/// constructors. Lower-level state inspection (needed for save/load)
/// will be added explicitly when the runtime needs it, rather than
/// leaked by exposing the underlying `ChaCha8Rng`.
///
/// # Examples
///
/// ```
/// use det_rng::DetRng;
/// use rand_core::Rng;
///
/// let mut a = DetRng::from_seed(42);
/// let mut b = DetRng::from_seed(42);
/// // Same seed → same output stream.
/// assert_eq!(a.next_u64(), b.next_u64());
/// ```
#[derive(Debug, Clone)]
pub struct DetRng {
    inner: ChaCha8Rng,
}

impl DetRng {
    /// Constructs a `DetRng` from a `u64` seed.
    ///
    /// The seed expands to `ChaCha8`'s 32-byte seed via the algorithm
    /// documented in [`rand_chacha`].
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: ChaCha8Rng::seed_from_u64(seed),
        }
    }
}

impl TryRng for DetRng {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::Rng;

    // ------------- splitmix64 -------------

    #[test]
    fn splitmix64_is_deterministic() {
        for x in [0_u64, 1, 42, 0xDEAD_BEEF, u64::MAX] {
            assert_eq!(splitmix64(x), splitmix64(x), "input {x}");
        }
    }

    #[test]
    fn splitmix64_zero_is_nonzero() {
        // splitmix64(0) must not be 0, or the mixer would have a
        // fixed point at zero.
        assert_ne!(splitmix64(0), 0);
    }

    #[test]
    fn splitmix64_matches_reference_vector() {
        // Reference output from Vigna's splitmix64.c with state=0
        // (i.e. splitmix64 applied to 0 once). If this changes, the
        // mixer constants have drifted and every derived seed in the
        // project shifts with them.
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
    }

    // ------------- derive_seed -------------

    #[test]
    fn derive_seed_is_deterministic() {
        assert_eq!(derive_seed(123, 456), derive_seed(123, 456));
    }

    #[test]
    fn derive_seed_differs_by_salt() {
        // The whole point: stage A and stage B with the same master
        // produce uncorrelated sub-seeds.
        let a = derive_seed(0xDEAD_BEEF, 1);
        let b = derive_seed(0xDEAD_BEEF, 2);
        assert_ne!(a, b);
    }

    #[test]
    fn derive_seed_differs_by_master() {
        let a = derive_seed(1, 42);
        let b = derive_seed(2, 42);
        assert_ne!(a, b);
    }

    #[test]
    fn derive_seed_avoids_xor_collision() {
        // The naive `splitmix64(master ^ salt)` would give
        // identical outputs whenever `master1 ^ salt1 == master2 ^
        // salt2`. Our two-stage mix must not have that collision.
        // (1 ^ 3) == (2 ^ 0) == 3, so the naive form would collide.
        let a = derive_seed(1, 3);
        let b = derive_seed(2, 0);
        assert_ne!(a, b);
    }

    // ------------- DetRng -------------

    #[test]
    fn det_rng_same_seed_produces_same_stream() {
        let mut a = DetRng::from_seed(7);
        let mut b = DetRng::from_seed(7);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn det_rng_different_seeds_diverge() {
        let mut a = DetRng::from_seed(7);
        let mut b = DetRng::from_seed(8);
        // Astronomically unlikely to collide on the first draw, but
        // assert across several to be safe against future seeding
        // changes in rand_chacha.
        let mut any_diff = false;
        for _ in 0..4 {
            if a.next_u64() != b.next_u64() {
                any_diff = true;
                break;
            }
        }
        assert!(any_diff);
    }

    #[test]
    fn det_rng_fill_bytes_fills_buffer() {
        let mut rng = DetRng::from_seed(0);
        let mut buf = [0_u8; 32];
        rng.fill_bytes(&mut buf);
        // Vanishingly unlikely to be all zero after a fill from a
        // real PRNG.
        assert!(buf.iter().any(|&b| b != 0));
    }
}
