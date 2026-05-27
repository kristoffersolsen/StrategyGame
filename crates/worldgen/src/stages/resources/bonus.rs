//! Bonus resource placement.

use crate::rng::StageRng;
use crate::world_data::ResourceData;

use super::filter::Eligibility;
use super::{PlacementContext, scatter};

/// Places all bonus resources by uniform Poisson-disk scatter.
pub(super) fn place_bonus_resources(
    context: &PlacementContext<'_>,
    eligibility: &Eligibility,
    resources: &mut ResourceData,
    rng: &mut StageRng,
) {
    for (resource, entry) in context.config.bonus_entries() {
        let eligible = scatter::eligible_tiles(eligibility, resource);
        let target = scatter::target_count(eligible.len(), entry, context.abundance);
        let mut deposits = Vec::new();
        let mut state = scatter::ScatterState {
            bounds: context.bounds,
            resources,
            params: context.params,
            rng,
        };
        scatter::place_uniform(
            resource,
            eligible,
            target,
            entry.min_spacing,
            &mut state,
            &mut deposits,
        );
    }
}
