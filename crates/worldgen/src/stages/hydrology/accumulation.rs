//! Precipitation-weighted flow accumulation.

use coords::{Coord, Direction, MapBounds};

use crate::Field;

/// Accumulates precipitation along the resolved single-flow drainage graph.
pub(crate) fn accumulate_flow(
    bounds: MapBounds,
    flow_direction: &Field<Option<Direction>>,
    elevation: &Field<f32>,
    height_level: &Field<u8>,
    is_lake: &Field<bool>,
    precipitation: &Field<f32>,
) -> Field<f32> {
    let mut accumulation = precipitation.clone();
    let mut coords: Vec<Coord> = elevation.iter_coords().map(|(coord, _)| coord).collect();
    coords.sort_by(|a, b| {
        elevation[*b]
            .total_cmp(&elevation[*a])
            .then_with(|| (a.r, a.q).cmp(&(b.r, b.q)))
    });

    for coord in coords {
        let Some(direction) = flow_direction[coord] else {
            continue;
        };
        let Some(downstream) = bounds.neighbor(coord, direction) else {
            continue;
        };
        accumulation.set(downstream, accumulation[downstream] + accumulation[coord]);
    }

    let max_land_flow = accumulation
        .iter_coords()
        .filter_map(|(coord, &flow)| (height_level[coord] > 1 && !is_lake[coord]).then_some(flow))
        .fold(0.0_f32, f32::max);

    if max_land_flow > 0.0 {
        for value in &mut accumulation {
            *value /= max_land_flow;
        }
    }

    accumulation
}
