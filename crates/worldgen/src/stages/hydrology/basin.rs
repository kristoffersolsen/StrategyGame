//! Basin detection and sink resolution.

use std::collections::{HashSet, VecDeque};

use coords::{Coord, Direction, MapBounds};

use crate::Field;
use crate::params::HydrologyParams;

use super::flow_direction::FLOW_DIRECTION_ORDER;

/// Tiny elevation lift used to route filled basins over their spill point.
const FILL_GRADIENT_EPSILON: f32 = 0.000_001;

/// A drainage basin terminating at a sink tile.
#[derive(Debug, Clone)]
pub(crate) struct Basin {
    tiles: Vec<Coord>,
    spill: Option<SpillPoint>,
}

/// Lowest outside neighbor through which a basin can spill.
#[derive(Debug, Clone, Copy)]
struct SpillPoint {
    inside: Coord,
    outside: Coord,
    elevation: f32,
}

/// Basin-resolution output consumed by downstream hydrology steps.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedBasins {
    /// Continuous elevation after small sink filling.
    pub(crate) elevation: Field<f32>,
    /// Discrete height level after sink filling and lake marking.
    pub(crate) height_level: Field<u8>,
    /// Lake tiles produced from large basins.
    pub(crate) is_lake: Field<bool>,
}

/// Detects all land basins that terminate in an initial sink tile.
pub(crate) fn detect_basins(
    bounds: MapBounds,
    elevation: &Field<f32>,
    flow_direction: &Field<Option<Direction>>,
    height_level: &Field<u8>,
) -> Vec<Basin> {
    let upstream = build_upstream_lists(bounds, flow_direction);
    let mut basins = Vec::new();

    for (coord, &flow) in flow_direction.iter_coords() {
        if height_level[coord] <= 1 || flow.is_some() {
            continue;
        }

        let tiles = collect_upstream_basin(bounds, coord, &upstream);
        let spill = find_spill_point(bounds, elevation, &tiles);
        basins.push(Basin { tiles, spill });
    }

    basins
}

/// Applies basin decisions to produce filled elevation, lakes, and
/// post-hydrology height levels.
pub(crate) fn resolve_basins(
    bounds: MapBounds,
    elevation: &Field<f32>,
    height_level: &Field<u8>,
    basins: &[Basin],
    params: &HydrologyParams,
) -> ResolvedBasins {
    let mut resolved_elevation = elevation.clone();
    let mut resolved_height = height_level.clone();
    let mut is_lake = Field::filled(bounds, false);

    for basin in basins {
        let Some(spill) = basin.spill else {
            mark_lake(&mut resolved_height, &mut is_lake, basin, height_level);
            continue;
        };

        if basin.tiles.len() >= params.lake_basin_area_threshold as usize {
            mark_lake_below_spill(
                &mut resolved_height,
                &mut is_lake,
                basin,
                spill.elevation,
                elevation,
                height_level,
            );
        } else {
            fill_small_basin(
                &mut resolved_elevation,
                &mut resolved_height,
                basin,
                spill,
                height_level,
            );
        }
    }

    ResolvedBasins {
        elevation: resolved_elevation,
        height_level: resolved_height,
        is_lake,
    }
}

fn build_upstream_lists(
    bounds: MapBounds,
    flow_direction: &Field<Option<Direction>>,
) -> Vec<Vec<Coord>> {
    let mut upstream = vec![Vec::new(); flow_direction.len()];
    for (coord, &flow) in flow_direction.iter_coords() {
        let Some(direction) = flow else {
            continue;
        };
        let Some(downstream) = bounds.neighbor(coord, direction) else {
            continue;
        };
        upstream[coord_index(bounds, downstream)].push(coord);
    }
    upstream
}

fn collect_upstream_basin(bounds: MapBounds, sink: Coord, upstream: &[Vec<Coord>]) -> Vec<Coord> {
    let mut tiles = Vec::new();
    let mut queue = VecDeque::from([sink]);

    while let Some(coord) = queue.pop_front() {
        tiles.push(coord);
        for &next in &upstream[coord_index(bounds, coord)] {
            queue.push_back(next);
        }
    }

    tiles
}

fn find_spill_point(
    bounds: MapBounds,
    elevation: &Field<f32>,
    basin_tiles: &[Coord],
) -> Option<SpillPoint> {
    let basin_set: HashSet<Coord> = basin_tiles.iter().copied().collect();
    let mut best = None;

    for &inside in basin_tiles {
        for direction in FLOW_DIRECTION_ORDER {
            let Some(outside) = bounds.neighbor(inside, direction) else {
                continue;
            };
            if basin_set.contains(&outside) {
                continue;
            }
            let candidate = SpillPoint {
                inside,
                outside,
                elevation: elevation[outside],
            };
            if spill_is_better(candidate, best) {
                best = Some(candidate);
            }
        }
    }

    best
}

fn spill_is_better(candidate: SpillPoint, current: Option<SpillPoint>) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate
        .elevation
        .total_cmp(&current.elevation)
        .then_with(|| coord_key(candidate.outside).cmp(&coord_key(current.outside)))
        .is_lt()
}

fn fill_small_basin(
    elevation: &mut Field<f32>,
    height_level: &mut Field<u8>,
    basin: &Basin,
    spill: SpillPoint,
    original_height: &Field<u8>,
) {
    let distances = distance_from_source(elevation.bounds(), basin, spill.inside);
    let spill_height = original_height[spill.outside];

    for &coord in &basin.tiles {
        let distance = distances[coord];
        #[allow(clippy::cast_precision_loss)]
        let lifted = spill.elevation + FILL_GRADIENT_EPSILON * (distance + 1) as f32;
        if elevation[coord] < lifted {
            elevation.set(coord, lifted);
        }
        height_level.set(coord, height_level[coord].max(spill_height));
    }

    let outlet_lift = spill.elevation + FILL_GRADIENT_EPSILON;
    elevation.set(spill.inside, outlet_lift);
}

fn distance_from_source(bounds: MapBounds, basin: &Basin, source: Coord) -> Field<u32> {
    let tile_set: HashSet<Coord> = basin.tiles.iter().copied().collect();
    let mut distances = Field::filled(bounds, u32::MAX);
    let mut queue = VecDeque::from([(source, 0_u32)]);

    while let Some((coord, distance)) = queue.pop_front() {
        if distances[coord] != u32::MAX {
            continue;
        }
        distances.set(coord, distance);
        for direction in FLOW_DIRECTION_ORDER {
            let Some(neighbor) = bounds.neighbor(coord, direction) else {
                continue;
            };
            if tile_set.contains(&neighbor) {
                queue.push_back((neighbor, distance + 1));
            }
        }
    }

    distances
}

fn mark_lake(
    height_level: &mut Field<u8>,
    is_lake: &mut Field<bool>,
    basin: &Basin,
    original_height: &Field<u8>,
) {
    for &coord in &basin.tiles {
        if original_height[coord] > 1 {
            is_lake.set(coord, true);
            height_level.set(coord, 1);
        }
    }
}

fn mark_lake_below_spill(
    height_level: &mut Field<u8>,
    is_lake: &mut Field<bool>,
    basin: &Basin,
    spill_elevation: f32,
    elevation: &Field<f32>,
    original_height: &Field<u8>,
) {
    for &coord in &basin.tiles {
        if original_height[coord] > 1 && elevation[coord] < spill_elevation {
            is_lake.set(coord, true);
            height_level.set(coord, 1);
        }
    }
}

fn coord_index(bounds: MapBounds, coord: Coord) -> usize {
    let width = bounds.width() as usize;
    let r = usize::try_from(coord.r).expect("normalized coord has non-negative r");
    let q = usize::try_from(coord.q).expect("normalized coord has non-negative q");
    r * width + q
}

fn coord_key(coord: Coord) -> (i32, i32) {
    (coord.r, coord.q)
}
