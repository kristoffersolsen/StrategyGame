//! Shared scatter placement helpers for resources.

use coords::{Coord, MapBounds};
use rand::RngExt;

use crate::params::ResourceParams;
use crate::rng::StageRng;
use crate::world_data::{MineralCharacter, ResourceData, ResourceId, TectonicData};

use super::config::ResourceEntry;
use super::filter::Eligibility;

/// Mutable state shared by scatter placement helpers.
pub(super) struct ScatterState<'a, 'b> {
    /// Map bounds used for spacing checks.
    pub(super) bounds: MapBounds,
    /// Resource output field being populated.
    pub(super) resources: &'a mut ResourceData,
    /// Placement tuning parameters.
    pub(super) params: &'a ResourceParams,
    /// Stage RNG.
    pub(super) rng: &'b mut StageRng,
}

/// Returns a resource's eligible tiles, or an empty slice if absent.
pub(super) fn eligible_tiles(eligibility: &Eligibility, resource: ResourceId) -> &[Coord] {
    eligibility
        .get(&resource)
        .map_or(&[] as &[Coord], Vec::as_slice)
}

/// Converts rarity and abundance to a concrete placement target.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub(super) fn target_count(eligible_count: usize, entry: &ResourceEntry, abundance: f32) -> usize {
    let target = (eligible_count as f32 * entry.rarity * abundance).round() as usize;
    target.min(eligible_count)
}

/// Places uniformly selected eligible tiles, respecting occupancy and spacing.
pub(super) fn place_uniform(
    resource: ResourceId,
    eligible: &[Coord],
    target: usize,
    min_spacing: u8,
    state: &mut ScatterState<'_, '_>,
    deposits: &mut Vec<Coord>,
) {
    if eligible.is_empty() || target == 0 {
        return;
    }
    let attempt_limit = attempt_limit(target, state.params);
    let mut attempts = 0_usize;
    while deposits.len() < target && attempts < attempt_limit {
        attempts += 1;
        let candidate = eligible[state.rng.random_range(0..eligible.len())];
        if can_place(
            state.bounds,
            candidate,
            min_spacing,
            state.resources,
            deposits,
        ) {
            state.resources.resource.set(candidate, Some(resource));
            deposits.push(candidate);
        }
    }
}

/// Places plate-weighted eligible tiles, respecting occupancy and spacing.
pub(super) fn place_weighted(
    tectonics: &TectonicData,
    resource: ResourceId,
    entry: &ResourceEntry,
    eligible: &[Coord],
    target: usize,
    state: &mut ScatterState<'_, '_>,
    deposits: &mut Vec<Coord>,
) {
    if eligible.is_empty() || target == 0 {
        return;
    }
    let attempt_limit = attempt_limit(target, state.params);
    let mut attempts = 0_usize;
    while deposits.len() < target && attempts < attempt_limit {
        attempts += 1;
        let candidate = choose_weighted(tectonics, entry, eligible, state.rng);
        if can_place(
            state.bounds,
            candidate,
            entry.min_spacing,
            state.resources,
            deposits,
        ) {
            state.resources.resource.set(candidate, Some(resource));
            deposits.push(candidate);
        }
    }
}

/// Returns `true` if the tile is empty and far enough from existing deposits.
pub(super) fn can_place(
    bounds: MapBounds,
    candidate: Coord,
    min_spacing: u8,
    resources: &ResourceData,
    deposits: &[Coord],
) -> bool {
    if resources.resource[candidate].is_some() {
        return false;
    }
    let min_spacing = i32::from(min_spacing);
    deposits
        .iter()
        .all(|&placed| bounds.distance(candidate, placed) >= min_spacing)
}

/// Selects one eligible coordinate using mineral-character weights.
pub(super) fn choose_weighted(
    tectonics: &TectonicData,
    entry: &ResourceEntry,
    eligible: &[Coord],
    rng: &mut StageRng,
) -> Coord {
    let total_weight: f32 = eligible
        .iter()
        .map(|&coord| mineral_weight(tectonics, entry, coord))
        .sum();
    if total_weight <= 0.0 {
        return eligible[rng.random_range(0..eligible.len())];
    }

    let mut roll = rng.random_range(0.0..total_weight);
    for &coord in eligible {
        let weight = mineral_weight(tectonics, entry, coord);
        if weight <= 0.0 {
            continue;
        }
        if roll <= weight {
            return coord;
        }
        roll -= weight;
    }
    *eligible
        .last()
        .expect("choose_weighted called with non-empty eligibility")
}

fn mineral_weight(tectonics: &TectonicData, entry: &ResourceEntry, coord: Coord) -> f32 {
    let plate_id = tectonics.plate_id[coord];
    let mineral = tectonics
        .plates
        .get(usize::from(plate_id))
        .map_or(MineralCharacter::Industrial, |plate| {
            plate.mineral_character
        });
    entry
        .mineral_affinities
        .get(&mineral)
        .copied()
        .unwrap_or(0.0)
        .max(0.0)
}

fn attempt_limit(target: usize, params: &ResourceParams) -> usize {
    target
        .saturating_mul(usize::from(params.poisson_max_attempts))
        .max(usize::from(params.poisson_max_attempts))
}
