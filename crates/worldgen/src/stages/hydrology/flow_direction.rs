//! D6 single-downstream flow direction.

use coords::{Direction, MapBounds};

use crate::Field;

/// Direction order used for hydrology tie-breaking.
///
/// This is the clockwise order recorded in ADR-0011. It is kept local
/// to hydrology so changing drainage tie-breaks does not alter the
/// general coordinate API's neighbor iteration order.
pub(crate) const FLOW_DIRECTION_ORDER: [Direction; 6] = [
    Direction::East,
    Direction::SouthEast,
    Direction::SouthWest,
    Direction::West,
    Direction::NorthWest,
    Direction::NorthEast,
];

/// Builds the downstream direction field for all non-terminal land tiles.
pub(crate) fn build_flow_direction(
    bounds: MapBounds,
    elevation: &Field<f32>,
    height_level: &Field<u8>,
    is_lake: &Field<bool>,
) -> Field<Option<Direction>> {
    Field::from_fn(bounds, |coord| {
        if height_level[coord] <= 1 || is_lake[coord] {
            return None;
        }

        let current = elevation[coord];
        let mut best = None;
        let mut best_elevation = current;
        for direction in FLOW_DIRECTION_ORDER {
            let Some(neighbor) = bounds.neighbor(coord, direction) else {
                continue;
            };
            let neighbor_elevation = elevation[neighbor];
            if neighbor_elevation < best_elevation {
                best = Some(direction);
                best_elevation = neighbor_elevation;
            }
        }
        best
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::Coord;

    #[test]
    fn chooses_lowest_neighbor() {
        let bounds = MapBounds::new(3, 3);
        let center = Coord::new(1, 1);
        let elevation = Field::from_fn(bounds, |coord| {
            if coord == center {
                10.0
            } else if coord == Coord::new(1, 2) {
                1.0
            } else {
                5.0
            }
        });
        let height_level = Field::filled(bounds, 2);
        let is_lake = Field::filled(bounds, false);

        let flow = build_flow_direction(bounds, &elevation, &height_level, &is_lake);

        assert_eq!(flow[center], Some(Direction::SouthEast));
    }

    #[test]
    fn breaks_ties_by_hydrology_direction_order() {
        let bounds = MapBounds::new(3, 3);
        let center = Coord::new(1, 1);
        let elevation = Field::from_fn(bounds, |coord| if coord == center { 10.0 } else { 1.0 });
        let height_level = Field::filled(bounds, 2);
        let is_lake = Field::filled(bounds, false);

        let flow = build_flow_direction(bounds, &elevation, &height_level, &is_lake);

        assert_eq!(flow[center], Some(Direction::East));
    }

    #[test]
    fn treats_water_and_lakes_as_terminals() {
        let bounds = MapBounds::new(3, 3);
        let elevation = Field::filled(bounds, 1.0);
        let mut height_level = Field::filled(bounds, 2);
        let mut is_lake = Field::filled(bounds, false);
        let water = Coord::new(0, 1);
        let lake = Coord::new(1, 1);
        height_level.set(water, 1);
        is_lake.set(lake, true);

        let flow = build_flow_direction(bounds, &elevation, &height_level, &is_lake);

        assert_eq!(flow[water], None);
        assert_eq!(flow[lake], None);
    }
}
