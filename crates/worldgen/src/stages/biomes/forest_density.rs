//! Forest-density feature generation.

use std::f64::consts::TAU;

use coords::{Coord, MapBounds};
use noise::{NoiseFn, OpenSimplex};

use crate::Field;
use crate::params::BiomeParams;
use crate::world_data::Biome;

const TORUS_RADIUS: f64 = 1.0 / TAU;
const NOISE_Z_SCALE: f64 = 1.0 / 150.0;

/// Builds a low-frequency forest-density field masked to forest biomes.
pub(super) fn build_forest_density(
    bounds: MapBounds,
    biome: &Field<Option<Biome>>,
    params: &BiomeParams,
    noise_seed: u32,
) -> Field<f32> {
    let noise = OpenSimplex::new(noise_seed);
    let width = f64::from(bounds.width());
    Field::from_fn(bounds, |coord| {
        let Some(tile_biome) = biome[coord] else {
            return 0.0;
        };
        if !tile_biome.is_forest() {
            return 0.0;
        }
        let sample = sample_noise(coord, width, params.forest_noise_frequency, &noise);
        let density = 0.5 + sample * params.forest_noise_amplitude * 0.5;
        density.clamp(0.0, 1.0)
    })
}

fn sample_noise(coord: Coord, map_width: f64, frequency: f32, noise: &OpenSimplex) -> f32 {
    let angle = TAU * f64::from(coord.q) / map_width;
    let nx = TORUS_RADIUS * angle.cos() * f64::from(frequency);
    let ny = TORUS_RADIUS * angle.sin() * f64::from(frequency);
    let nz = f64::from(coord.r) * NOISE_Z_SCALE * f64::from(frequency);
    #[allow(clippy::cast_possible_truncation)]
    {
        noise.get([nx, ny, nz]) as f32
    }
}
