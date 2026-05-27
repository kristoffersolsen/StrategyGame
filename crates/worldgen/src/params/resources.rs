//! Tuning parameters for the resource placement stage (Stage 3.5, design §1.13).
//!
//! Owns scalar knobs consumed by the resource scatter and cluster
//! placement algorithms. Per-resource content lives in the RON data
//! asset loaded by the resource stage.

use crate::error::GenerationError;

const DEFAULT_CLUSTER_EXPANSION_ATTEMPTS: u8 = 8;
const DEFAULT_POISSON_MAX_ATTEMPTS: u8 = 30;

const MIN_CLUSTER_EXPANSION_ATTEMPTS: u8 = 1;
const MAX_CLUSTER_EXPANSION_ATTEMPTS: u8 = 32;
const MIN_POISSON_MAX_ATTEMPTS: u8 = 5;
const MAX_POISSON_MAX_ATTEMPTS: u8 = 100;

/// Tuning parameters for resource placement.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceParams {
    /// Neighbor candidates tried per clustered luxury seed.
    ///
    /// Valid range: `1..=32`. Default: 8.
    pub cluster_expansion_attempts: u8,

    /// Rejection-sampling attempts per desired Poisson-disk placement.
    ///
    /// Valid range: `5..=100`. Default: 30.
    pub poisson_max_attempts: u8,
}

impl ResourceParams {
    /// Returns the default resource parameters.
    #[must_use]
    pub const fn defaults() -> Self {
        Self {
            cluster_expansion_attempts: DEFAULT_CLUSTER_EXPANSION_ATTEMPTS,
            poisson_max_attempts: DEFAULT_POISSON_MAX_ATTEMPTS,
        }
    }

    /// Validates all field ranges.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::InvalidParams`] if any field falls
    /// outside its documented range.
    pub fn validate(&self) -> Result<(), GenerationError> {
        if !(MIN_CLUSTER_EXPANSION_ATTEMPTS..=MAX_CLUSTER_EXPANSION_ATTEMPTS)
            .contains(&self.cluster_expansion_attempts)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "cluster_expansion_attempts {} must be in \
                     {MIN_CLUSTER_EXPANSION_ATTEMPTS}..={MAX_CLUSTER_EXPANSION_ATTEMPTS}",
                    self.cluster_expansion_attempts
                ),
            });
        }
        if !(MIN_POISSON_MAX_ATTEMPTS..=MAX_POISSON_MAX_ATTEMPTS)
            .contains(&self.poisson_max_attempts)
        {
            return Err(GenerationError::InvalidParams {
                reason: format!(
                    "poisson_max_attempts {} must be in \
                     {MIN_POISSON_MAX_ATTEMPTS}..={MAX_POISSON_MAX_ATTEMPTS}",
                    self.poisson_max_attempts
                ),
            });
        }
        Ok(())
    }

    /// Returns `true` if every field equals its default value.
    #[must_use]
    pub fn is_all_default(&self) -> bool {
        *self == Self::defaults()
    }
}

impl Default for ResourceParams {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        ResourceParams::defaults()
            .validate()
            .expect("defaults must always validate");
    }

    #[test]
    fn rejects_too_few_poisson_attempts() {
        let params = ResourceParams {
            poisson_max_attempts: MIN_POISSON_MAX_ATTEMPTS - 1,
            ..ResourceParams::defaults()
        };
        assert!(matches!(
            params.validate(),
            Err(GenerationError::InvalidParams { .. })
        ));
    }
}
