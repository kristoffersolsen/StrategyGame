//! Strategic resource placement.

use crate::rng::StageRng;
use crate::world_data::ResourceData;

use super::filter::Eligibility;
use super::{PlacementContext, scatter};

/// Places all strategic resources with baseline and concentration passes.
pub(super) fn place_strategic_resources(
    context: &PlacementContext<'_>,
    eligibility: &Eligibility,
    resources: &mut ResourceData,
    rng: &mut StageRng,
) {
    for (resource, entry) in context.config.strategic_entries() {
        let eligible = scatter::eligible_tiles(eligibility, resource);
        let target = scatter::target_count(eligible.len(), entry, context.abundance);
        let baseline_target = baseline_target(target, entry.baseline_fraction);
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
            baseline_target,
            entry.min_spacing,
            &mut state,
            &mut deposits,
        );
        scatter::place_weighted(
            context.tectonics,
            resource,
            entry,
            eligible,
            target,
            &mut state,
            &mut deposits,
        );
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn baseline_target(total: usize, baseline_fraction: f32) -> usize {
    (total as f32 * baseline_fraction).round() as usize
}
