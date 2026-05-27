//! Luxury resource placement.

use coords::Coord;
use rand::RngExt;

use crate::rng::StageRng;
use crate::world_data::{ResourceData, ResourceId, TectonicData};

use super::config::ResourceEntry;
use super::filter::Eligibility;
use super::{PlacementContext, scatter};

const MIN_CLUSTER_SIZE: usize = 2;
const MAX_CLUSTER_SIZE: usize = 4;

/// Places all luxury resources with a small baseline and clustered concentration pass.
pub(super) fn place_luxury_resources(
    context: &PlacementContext<'_>,
    eligibility: &Eligibility,
    resources: &mut ResourceData,
    rng: &mut StageRng,
) {
    for (resource, entry) in context.config.luxury_entries() {
        let eligible = scatter::eligible_tiles(eligibility, resource);
        let target = scatter::target_count(eligible.len(), entry, context.abundance);
        let baseline_target = baseline_target(target, entry.baseline_fraction);
        let mut seed_deposits = Vec::new();
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
            &mut seed_deposits,
        );

        place_clustered_concentration(
            context.tectonics,
            resource,
            entry,
            eligible,
            target,
            &mut state,
            &mut seed_deposits,
        );
    }
}

fn place_clustered_concentration(
    tectonics: &TectonicData,
    resource: ResourceId,
    entry: &ResourceEntry,
    eligible: &[Coord],
    target: usize,
    state: &mut scatter::ScatterState<'_, '_>,
    seed_deposits: &mut Vec<Coord>,
) {
    if eligible.is_empty() || seed_deposits.len() >= target {
        return;
    }

    let attempt_limit = target
        .saturating_mul(usize::from(state.params.poisson_max_attempts))
        .max(usize::from(state.params.poisson_max_attempts));
    let mut attempts = 0_usize;
    let mut placed_count = seed_deposits.len();
    while placed_count < target && attempts < attempt_limit {
        attempts += 1;
        let seed = scatter::choose_weighted(tectonics, entry, eligible, state.rng);
        if !scatter::can_place(
            state.bounds,
            seed,
            entry.min_spacing,
            state.resources,
            seed_deposits,
        ) {
            continue;
        }

        state.resources.resource.set(seed, Some(resource));
        seed_deposits.push(seed);
        placed_count += 1;

        let remaining = target - placed_count;
        if remaining == 0 {
            break;
        }
        let desired_cluster_size = state
            .rng
            .random_range(MIN_CLUSTER_SIZE..=MAX_CLUSTER_SIZE)
            .min(remaining + 1);
        let expansion_target = desired_cluster_size - 1;
        placed_count += expand_cluster(resource, seed, entry, eligible, expansion_target, state);
    }
}

fn expand_cluster(
    resource: ResourceId,
    seed: Coord,
    entry: &ResourceEntry,
    eligible: &[Coord],
    target: usize,
    state: &mut scatter::ScatterState<'_, '_>,
) -> usize {
    if target == 0 {
        return 0;
    }
    let candidates = state.bounds.range(seed, i32::from(entry.cluster_radius));
    let mut placed = 0_usize;
    let mut attempts = 0_u8;
    while placed < target && attempts < state.params.cluster_expansion_attempts {
        attempts += 1;
        let candidate = candidates[state.rng.random_range(0..candidates.len())];
        if eligible.contains(&candidate) && state.resources.resource[candidate].is_none() {
            state.resources.resource.set(candidate, Some(resource));
            placed += 1;
        }
    }
    placed
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn baseline_target(total: usize, baseline_fraction: f32) -> usize {
    (total as f32 * baseline_fraction).round() as usize
}
