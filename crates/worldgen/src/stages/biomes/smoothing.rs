//! Conservative smoothing for isolated biome islands.

use std::collections::BTreeMap;

use coords::MapBounds;

use crate::Field;
use crate::params::BiomeParams;
use crate::world_data::{BiomeHandle, HydrologyData};

// Three neighbors required for majority replacement: conservative threshold
// that prevents over-smoothing. Could be a BiomeParams field, but the
// existing smoothing_passes param already controls aggressiveness.
const MIN_MAJORITY_NEIGHBORS: u8 = 3;

/// Runs the configured number of smoothing passes.
pub(super) fn smooth_biomes(
    bounds: MapBounds,
    biome: &Field<Option<BiomeHandle>>,
    hydrology: &HydrologyData,
    params: &BiomeParams,
) -> Field<Option<BiomeHandle>> {
    let mut current = biome.clone();
    for _ in 0..params.smoothing_passes {
        current = smooth_once(bounds, &current, hydrology);
    }
    current
}

fn smooth_once(
    bounds: MapBounds,
    biome: &Field<Option<BiomeHandle>>,
    hydrology: &HydrologyData,
) -> Field<Option<BiomeHandle>> {
    Field::from_fn(bounds, |coord| {
        let current = biome[coord]?;

        let mut same_count = 0_u8;
        // BiomeHandle implements Ord, so BTreeMap works directly.
        let mut counts = BTreeMap::<BiomeHandle, u8>::new();
        for neighbor in bounds.neighbors(coord).into_iter().flatten() {
            if hydrology.is_lake[neighbor] {
                continue;
            }
            let Some(neighbor_biome) = biome[neighbor] else {
                continue;
            };
            if neighbor_biome == current {
                same_count += 1;
            }
            *counts.entry(neighbor_biome).or_default() += 1;
        }

        if same_count > 0 {
            return Some(current);
        }

        let replacement = counts
            .into_iter()
            .filter(|(_, count)| *count >= MIN_MAJORITY_NEIGHBORS)
            .max_by_key(|(_, count)| *count)
            .map(|(candidate, _)| candidate);

        replacement.or(Some(current))
    })
}
