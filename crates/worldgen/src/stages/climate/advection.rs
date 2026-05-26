//! Moisture advection step kernel (step 4).
//!
//! Implements one iteration of the donor-cell upwind scheme for
//! moisture transport on the hex grid. The scheme is described in
//! ADR-0005 and the `STATUS.md` §3.2 mathematical notes.
//!
//! # Algorithm
//!
//! For each tile (visited in row-major `(r, q)` order for determinism):
//!
//! 1. For each of the six hex neighbours, compute the outflow projection:
//!    `p_k = max(0, dot(wind[tile], unit_k))`.
//! 2. The total outflow fraction `W = Σ p_k`.
//! 3. The tile retains fraction `(1 − W)` of its moisture.
//! 4. Each neighbour receives fraction `p_k` of the tile's moisture.
//! 5. The evaporation source is added to `M_next[tile]`.
//!
//! With `|wind| ≤ 0.25` the maximum `W ≤ 0.5` (half the strict CFL
//! bound), so `(1 − W) ≥ 0.5` and no tile ever loses all its moisture
//! in one step.
//!
//! # Double buffering
//!
//! The caller provides both `moisture_in` (read-only source field)
//! and `moisture_out` (write-only accumulation buffer, pre-zeroed).
//! This separates read and write so the iteration order does not affect
//! the result — it affects only floating-point rounding, which is
//! deterministic for a fixed `(r, q)` order with single-threaded code.

use coords::{Coord, Direction, MapBounds};

use crate::Field;
use crate::params::ClimateParams;
use crate::stages::climate::wind::HEX_UNIT_VECTORS;

/// Height levels above this value are land tiles (ocean = 0, coast = 1, land ≥ 2).
const LAND_LEVEL_MIN: u8 = 2;

// ------------- Direction-to-unit-vector mapping -------------

/// Maps each [`Direction`] to its index in [`HEX_UNIT_VECTORS`].
///
/// `Direction::ALL` order: [`East`][coords::Direction::East],
/// [`West`][coords::Direction::West], [`NorthEast`][coords::Direction::NorthEast],
/// [`NorthWest`][coords::Direction::NorthWest], [`SouthEast`][coords::Direction::SouthEast],
/// [`SouthWest`][coords::Direction::SouthWest]. [`HEX_UNIT_VECTORS`] uses the same order,
/// so the index is the position in `Direction::ALL`.
const DIRECTION_UNIT_VEC: [(Direction, usize); 6] = [
    (Direction::East, 0),
    (Direction::West, 1),
    (Direction::NorthEast, 2),
    (Direction::NorthWest, 3),
    (Direction::SouthEast, 4),
    (Direction::SouthWest, 5),
];

// ------------- Public entry point -------------

/// Performs one donor-cell upwind advection step.
///
/// Reads from `moisture_in`, writes into `moisture_out` (which the
/// caller must have zeroed before calling). Also adds the per-tile
/// evaporation source and — for land tiles — the evapotranspiration
/// re-emission to the output.
///
/// Moisture that would flow off the north or south pole is dropped
/// (no wrap in `r`). East-west wrap is handled automatically by
/// [`MapBounds::neighbor`].
///
/// # Arguments
///
/// * `moisture_in` — moisture field at the start of this iteration.
/// * `moisture_out` — pre-zeroed accumulation buffer; filled by this call.
/// * `wind` — per-tile 2D wind vectors from step 2.
/// * `evaporation` — per-tile evaporation sources from step 3.
/// * `height_level` — discrete height level from the tectonics stage.
/// * `bounds` — map dimensions.
/// * `params` — climate tuning parameters.
pub(crate) fn advection_step(
    moisture_in: &Field<f32>,
    moisture_out: &mut Field<f32>,
    wind: &Field<[f32; 2]>,
    evaporation: &Field<f32>,
    height_level: &Field<u8>,
    bounds: MapBounds,
    params: &ClimateParams,
) {
    let w = bounds.width().cast_signed();
    let h = bounds.height().cast_signed();

    for r in 0..h {
        for q in 0..w {
            let coord = Coord::new(q, r);
            let m = moisture_in[coord];
            let [wx, wy] = wind[coord];

            // Compute outflow projections onto each neighbour direction.
            let mut total_outflow = 0.0_f32;
            let mut projections = [0.0_f32; 6];

            for (_dir, idx) in DIRECTION_UNIT_VEC {
                let [ux, uy] = HEX_UNIT_VECTORS[idx];
                let proj = wx * ux + wy * uy;
                if proj > 0.0 {
                    projections[idx] = proj;
                    total_outflow += proj;
                }
            }

            // Retained fraction: what the tile keeps after outflow.
            // With |wind| ≤ 0.25, total_outflow ≤ 0.5, so retained ≥ 0.5.
            let retained = m * (1.0 - total_outflow).max(0.0);
            *moisture_out.get_mut(coord).expect("coord is in bounds") += retained;

            // Distribute outflow to in-bounds neighbours via public API.
            for (dir, idx) in DIRECTION_UNIT_VEC {
                if projections[idx] <= 0.0 {
                    continue;
                }
                if let Some(neighbour) = bounds.neighbor(coord, dir) {
                    *moisture_out
                        .get_mut(neighbour)
                        .expect("normalized neighbor is in bounds") += m * projections[idx];
                }
                // Moisture flowing off the north/south edge is lost (poles).
            }

            // Add evaporation source.
            *moisture_out.get_mut(coord).expect("coord is in bounds") += evaporation[coord];

            // Evapotranspiration: land tiles recycle a fraction of their moisture
            // back into the atmosphere, allowing it to travel further inland.
            if params.evapotranspiration_rate > 0.0 && height_level[coord] >= LAND_LEVEL_MIN {
                *moisture_out.get_mut(coord).expect("coord is in bounds") +=
                    params.evapotranspiration_rate * m;
            }

            // Moisture cap: prevent any tile from accumulating unbounded moisture.
            #[allow(clippy::float_cmp)]
            if params.moisture_cap != f32::MAX {
                let out = moisture_out.get_mut(coord).expect("coord is in bounds");
                *out = out.min(params.moisture_cap);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    #[allow(clippy::type_complexity)]
    fn make_fields(
        bounds: MapBounds,
    ) -> (
        Field<f32>,
        Field<f32>,
        Field<[f32; 2]>,
        Field<f32>,
        Field<u8>,
    ) {
        let moisture_in = Field::filled(bounds, 0.0_f32);
        let moisture_out = Field::filled(bounds, 0.0_f32);
        let wind = Field::filled(bounds, [0.0_f32, 0.0]);
        let evap = Field::filled(bounds, 0.0_f32);
        let height_level = Field::filled(bounds, 0_u8); // all ocean: no ET
        (moisture_in, moisture_out, wind, evap, height_level)
    }

    #[test]
    fn zero_wind_retains_all_moisture() {
        let bounds = MapBounds::new(10, 10);
        let (mut moisture_in, mut moisture_out, wind, evap, height_level) = make_fields(bounds);
        moisture_in.set(Coord::new(5, 5), 1.0);

        let params = ClimateParams::defaults();
        advection_step(
            &moisture_in,
            &mut moisture_out,
            &wind,
            &evap,
            &height_level,
            bounds,
            &params,
        );

        let at_center = *moisture_out.get(Coord::new(5, 5)).expect("in bounds");
        let total: f32 = moisture_out.iter().copied().sum();
        assert!(
            (at_center - 1.0).abs() < 1e-5,
            "zero wind: center should keep moisture, got {at_center}"
        );
        assert!(
            (total - 1.0).abs() < 1e-5,
            "zero wind: total moisture should be conserved, got {total}"
        );
    }

    #[test]
    fn evaporation_is_added_to_output() {
        let bounds = MapBounds::new(5, 5);
        let (moisture_in, mut moisture_out, wind, mut evap, height_level) = make_fields(bounds);
        evap.set(Coord::new(2, 2), 0.1);

        let params = ClimateParams::defaults();
        advection_step(
            &moisture_in,
            &mut moisture_out,
            &wind,
            &evap,
            &height_level,
            bounds,
            &params,
        );

        let v = *moisture_out.get(Coord::new(2, 2)).expect("in bounds");
        assert!((v - 0.1).abs() < 1e-6, "evaporation must be added, got {v}");
    }

    #[test]
    fn positive_x_wind_distributes_to_east_and_northeast_neighbors() {
        let bounds = MapBounds::new(10, 10);
        let (mut moisture_in, mut moisture_out, mut wind, evap, height_level) = make_fields(bounds);
        moisture_in.set(Coord::new(5, 5), 1.0);
        // Wind with only positive x component — projects onto East (idx 0)
        // and NorthEast (idx 2) which both have positive x components.
        wind.fill([0.1_f32, 0.0]);

        let params = ClimateParams::defaults();
        advection_step(
            &moisture_in,
            &mut moisture_out,
            &wind,
            &evap,
            &height_level,
            bounds,
            &params,
        );

        let total: f32 = moisture_out.iter().copied().sum();
        // Moisture is conserved (no polar loss for interior tile).
        assert!(
            (total - 1.0).abs() < 1e-4,
            "total moisture must be conserved for interior tile, got {total}"
        );
    }
}
