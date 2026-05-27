//! Typed errors produced by the generation pipeline.

use thiserror::Error;

/// Errors that can be returned by [`crate::generate_world`].
///
/// New variants will be added as later pipeline stages are implemented.
/// The variants currently defined are those reachable from Stage 2's
/// parameter-validation path; stage-specific failures (numerical
/// instability, adequacy failure, etc.) are added by the stage that
/// can produce them.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GenerationError {
    /// The supplied [`crate::WorldParams`] are internally inconsistent
    /// and cannot be used to drive generation.
    ///
    /// The `reason` field is a short human-readable message intended
    /// for logs and developer-facing diagnostics, not end-user UI.
    #[error("invalid generation parameters: {reason}")]
    InvalidParams {
        /// Human-readable description of which parameter is wrong and
        /// why.
        reason: String,
    },

    /// The biome lookup table could not be loaded or failed validation.
    #[error("invalid biome table {path}: {reason}")]
    InvalidBiomeTable {
        /// Path to the table being loaded.
        path: String,
        /// Human-readable description of the table problem.
        reason: String,
    },

    /// Starting-position selection (Stage 3.6, design §1.14) could not
    /// place all requested players while meeting the adequacy floor
    /// and spacing constraints.
    ///
    /// This variant is declared but unreachable until Stage 3.6 is
    /// implemented.
    #[error("starting-position adequacy floor could not be satisfied for {players} players")]
    AdequacyFailure {
        /// Number of players the generator was asked to place.
        players: u32,
    },
}
