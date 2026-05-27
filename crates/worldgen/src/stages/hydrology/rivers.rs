//! River tier identification from normalised flow accumulation.

use crate::Field;
use crate::params::HydrologyParams;
use crate::world_data::RiverSize;

/// Classifies river tiles into creek, river, and major-river tiers.
pub(crate) fn classify_rivers(
    flow_accumulation: &Field<f32>,
    height_level: &Field<u8>,
    is_lake: &Field<bool>,
    params: &HydrologyParams,
) -> Field<Option<RiverSize>> {
    let river_threshold = (params.river_flow_threshold + params.major_river_flow_threshold) * 0.5;

    Field::from_fn(flow_accumulation.bounds(), |coord| {
        if height_level[coord] <= 1 || is_lake[coord] {
            return None;
        }

        let flow = flow_accumulation[coord];
        if flow >= params.major_river_flow_threshold {
            Some(RiverSize::MajorRiver)
        } else if flow >= river_threshold {
            Some(RiverSize::River)
        } else if flow >= params.river_flow_threshold {
            Some(RiverSize::Creek)
        } else {
            None
        }
    })
}
