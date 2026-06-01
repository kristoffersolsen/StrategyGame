//! Starting position selection (Stage 3.6, design §1.14).
//!
//! Computes a per-tile viability score from biome yield proxies and
//! freshwater/terrain bonuses, then selects N start positions greedily
//! by descending neighbourhood score subject to a minimum spacing
//! constraint.

use std::collections::BTreeMap;

use coords::{Coord, MapBounds};

use crate::{
    Field, GenerationError,
    params::StartingPositionParams,
    rng::StageRng,
    world_data::{BiomeData, BiomeRegistry, HydrologyData, StartingPositionData, TectonicData},
};

// ------------- Public entry point -------------

/// Inputs consumed by the starting position stage.
pub(crate) struct StartingPositionInputs<'a> {
    pub(crate) tectonics: &'a TectonicData,
    pub(crate) hydrology: &'a HydrologyData,
    pub(crate) biomes: &'a BiomeData,
    pub(crate) biome_registry: &'a BiomeRegistry,
}

/// Runs the starting position selection stage end-to-end.
///
/// Builds a per-tile viability field, then greedily selects
/// `player_count` positions by descending score while enforcing the
/// minimum spacing from [`StartingPositionParams::min_spacing`].
///
/// # Errors
///
/// Returns [`GenerationError::AdequacyFailure`] when fewer than
/// `player_count` eligible tiles exist after spacing constraints are
/// applied.
pub(crate) fn run(
    inputs: &StartingPositionInputs<'_>,
    params: &StartingPositionParams,
    player_count: u32,
    _rng: &mut StageRng,
) -> Result<StartingPositionData, GenerationError> {
    let bounds = inputs.tectonics.height_level.bounds();

    let biome_food = resolve_biome_scores(inputs.biome_registry, &params.biome_food_scores);
    let biome_production =
        resolve_biome_scores(inputs.biome_registry, &params.biome_production_scores);

    let base_yield = compute_base_yield(
        bounds,
        inputs.tectonics,
        inputs.hydrology,
        inputs.biomes,
        &biome_food,
        &biome_production,
        params,
    );

    let viability = compute_viability(bounds, &base_yield, params.neighborhood_radius);

    let start_positions = select_positions(
        bounds,
        inputs.tectonics,
        inputs.hydrology,
        inputs.biomes,
        &viability,
        params,
        player_count,
    )?;

    Ok(StartingPositionData {
        start_positions,
        viability,
    })
}

// ------------- Viability scoring -------------

/// Resolves biome string-ID → score map to a dense handle-indexed vec.
///
/// String IDs are resolved once at this boundary; all per-tile scoring
/// uses handle indices thereafter.
fn resolve_biome_scores(registry: &BiomeRegistry, scores: &BTreeMap<String, f32>) -> Vec<f32> {
    let n = registry.inner.len();
    let mut arr = vec![0.0_f32; n];
    for handle in registry.all_handles() {
        let id = registry.id_of(handle);
        arr[handle.index()] = scores.get(id).copied().unwrap_or(0.0);
    }
    arr
}

/// Computes the per-tile base yield: weighted food + production proxy.
///
/// - Non-biome tiles (ocean, coast, mountain, lake) contribute 0.
/// - Floodplain and river tiles receive an extra food bonus.
/// - Hilly tiles receive an extra production bonus.
fn compute_base_yield(
    bounds: MapBounds,
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    biomes: &BiomeData,
    biome_food: &[f32],
    biome_production: &[f32],
    params: &StartingPositionParams,
) -> Field<f32> {
    Field::from_fn(bounds, |coord| {
        let Some(biome_handle) = biomes.biome[coord] else {
            return 0.0;
        };

        let food = biome_food[biome_handle.index()];
        let production = biome_production[biome_handle.index()];

        let freshwater_bonus =
            if hydrology.is_floodplain[coord] || hydrology.river_size[coord].is_some() {
                params.freshwater_food_bonus
            } else {
                0.0
            };

        let hill_bonus = if tectonics.is_hilly[coord] {
            params.hilly_production_bonus
        } else {
            0.0
        };

        params.food_weight * (food + freshwater_bonus)
            + params.production_weight * (production + hill_bonus)
    })
}

/// Sums per-tile base yield over a neighbourhood of `radius` hex steps.
///
/// Each tile's score is the total yield of all in-bounds tiles within
/// `radius` steps, including itself.
fn compute_viability(bounds: MapBounds, base_yield: &Field<f32>, radius: u32) -> Field<f32> {
    let r = i32::try_from(radius).expect("radius fits in i32");
    Field::from_fn(bounds, |coord| {
        bounds
            .range(coord, r)
            .into_iter()
            .map(|nc| base_yield[nc])
            .sum()
    })
}

// ------------- Position selection -------------

/// Greedily selects up to `player_count` positions by descending viability
/// subject to `params.adequacy_floor` and `params.min_spacing`.
fn select_positions(
    bounds: MapBounds,
    tectonics: &TectonicData,
    hydrology: &HydrologyData,
    biomes: &BiomeData,
    viability: &Field<f32>,
    params: &StartingPositionParams,
    player_count: u32,
) -> Result<Vec<Coord>, GenerationError> {
    // Collect eligible candidates: land tiles with biome and above the floor.
    let mut candidates: Vec<(Coord, f32)> = viability
        .iter_coords()
        .filter_map(|(coord, &score)| {
            let hl = tectonics.height_level[coord];
            if hl <= 1 {
                return None; // ocean or coast
            }
            if hydrology.is_lake[coord] {
                return None;
            }
            biomes.biome[coord]?;
            if score < params.adequacy_floor {
                return None;
            }
            Some((coord, score))
        })
        .collect();

    // Sort by descending score; deterministic tie-breaking by coord (q, then r).
    candidates.sort_unstable_by(|(ca, sa), (cb, sb)| {
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| ca.q.cmp(&cb.q))
            .then_with(|| ca.r.cmp(&cb.r))
    });

    let min_spacing = i32::try_from(params.min_spacing).expect("min_spacing fits in i32");
    let mut selected: Vec<Coord> = Vec::with_capacity(player_count as usize);

    'outer: for (coord, _score) in &candidates {
        for &existing in &selected {
            if bounds.distance(*coord, existing) < min_spacing {
                continue 'outer;
            }
        }
        selected.push(*coord);
        if selected.len() == player_count as usize {
            break;
        }
    }

    if selected.len() < player_count as usize {
        return Err(GenerationError::AdequacyFailure {
            players: player_count,
        });
    }

    Ok(selected)
}
