//! Placeholder terrain colour lookup for P1 vertex-colour rendering.

use bevy::color::{ColorToComponents, LinearRgba};
use coords::Coord;
use worldgen::{Biome, WorldData};

const OCEAN: LinearRgba = hex_color(0x1a, 0x3a, 0x5c);
const COAST: LinearRgba = hex_color(0x2e, 0x6e, 0xa6);
const MOUNTAIN: LinearRgba = hex_color(0x5a, 0x5a, 0x5a);
const RIVER_TINT: LinearRgba = hex_color(0x2c, 0x8f, 0xd8);

/// Returns the flat RGBA colour used for a tile in the P1 renderer.
pub(crate) fn tile_color(world: &WorldData, coord: Coord, mountain_level: u8) -> [f32; 4] {
    let height_level = *world
        .tectonics
        .height_level
        .get(coord)
        .expect("rendered coord must be in bounds");

    let mut color = match height_level {
        0 => OCEAN,
        1 => COAST,
        level if level >= mountain_level => MOUNTAIN,
        _ => {
            let biome = world.biomes.biome.get(coord).copied().flatten();
            let mut color = biome.map_or(hex_color(0x7a, 0x9a, 0x3a), biome_color);
            if world
                .tectonics
                .is_hilly
                .get(coord)
                .copied()
                .unwrap_or(false)
            {
                color = darken(color, 0.85);
            }
            color
        }
    };

    if world
        .hydrology
        .river_size
        .get(coord)
        .copied()
        .flatten()
        .is_some()
    {
        color = mix(color, RIVER_TINT, 0.25);
    }

    color.to_f32_array()
}

fn biome_color(biome: Biome) -> LinearRgba {
    match biome {
        Biome::TropicalRainforest => hex_color(0x2d, 0x6a, 0x2d),
        Biome::TemperateForest => hex_color(0x4a, 0x7c, 0x4a),
        Biome::BorealForest => hex_color(0x4a, 0x6b, 0x55),
        Biome::Grassland => hex_color(0x7a, 0x9a, 0x3a),
        Biome::Desert => hex_color(0xc8, 0xa9, 0x6e),
        Biome::Tundra => hex_color(0x8a, 0x9a, 0x8a),
    }
}

const fn hex_color(red: u8, green: u8, blue: u8) -> LinearRgba {
    LinearRgba {
        red: red as f32 / 255.0,
        green: green as f32 / 255.0,
        blue: blue as f32 / 255.0,
        alpha: 1.0,
    }
}

fn darken(color: LinearRgba, factor: f32) -> LinearRgba {
    LinearRgba {
        red: color.red * factor,
        green: color.green * factor,
        blue: color.blue * factor,
        alpha: color.alpha,
    }
}

fn mix(base: LinearRgba, overlay: LinearRgba, amount: f32) -> LinearRgba {
    let inverse = 1.0 - amount;
    LinearRgba {
        red: base.red * inverse + overlay.red * amount,
        green: base.green * inverse + overlay.green * amount,
        blue: base.blue * inverse + overlay.blue * amount,
        alpha: base.alpha,
    }
}
