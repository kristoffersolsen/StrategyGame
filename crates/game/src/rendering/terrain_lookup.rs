//! Placeholder terrain lookup for P1/P2 terrain rendering.

use bevy::color::{ColorToComponents, LinearRgba};
use coords::Coord;
use worldgen::{Biome, WorldData};

pub(crate) const TEXTURE_SIZE: u32 = 64;
pub(crate) const TEXTURE_LAYER_COUNT: u32 = 18;

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

/// Returns the P2 texture-array layer index used for a tile.
pub(crate) fn texture_index(world: &WorldData, coord: Coord, mountain_level: u8) -> u32 {
    let height_level = *world
        .tectonics
        .height_level
        .get(coord)
        .expect("rendered coord must be in bounds");

    match height_level {
        0 => 0,
        1 => 1,
        level if level >= mountain_level => {
            if is_hilly(world, coord) {
                17
            } else {
                8
            }
        }
        _ => {
            let biome = world.biomes.biome.get(coord).copied().flatten();
            let base = match biome.unwrap_or(Biome::Grassland) {
                Biome::TropicalRainforest => 2,
                Biome::TemperateForest => 3,
                Biome::BorealForest => 4,
                Biome::Grassland => 5,
                Biome::Desert => 6,
                Biome::Tundra => 7,
            };

            if is_hilly(world, coord) {
                base + 9
            } else {
                base
            }
        }
    }
}

/// Builds the flat-colour placeholder texture array used by the P2 renderer.
pub(crate) fn placeholder_texture_array_data() -> Vec<u8> {
    let layer_len = usize::try_from(TEXTURE_SIZE * TEXTURE_SIZE * 4)
        .expect("placeholder texture layer size fits in usize");
    let mut data = Vec::with_capacity(
        layer_len * usize::try_from(TEXTURE_LAYER_COUNT).expect("layer count fits in usize"),
    );

    for layer in 0..TEXTURE_LAYER_COUNT {
        let color = texture_layer_color(layer);
        let border = darken(color, 0.70);
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let pixel = if x < 4 || y < 4 || x >= TEXTURE_SIZE - 4 || y >= TEXTURE_SIZE - 4 {
                    border
                } else {
                    color
                };
                data.extend_from_slice(&rgba8(pixel));
            }
        }
    }

    data
}

fn is_hilly(world: &WorldData, coord: Coord) -> bool {
    world
        .tectonics
        .is_hilly
        .get(coord)
        .copied()
        .unwrap_or(false)
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

fn texture_layer_color(layer: u32) -> LinearRgba {
    match layer {
        0 | 9 => OCEAN,
        1 | 10 => COAST,
        2 | 11 => biome_color(Biome::TropicalRainforest),
        3 | 12 => biome_color(Biome::TemperateForest),
        4 | 13 => biome_color(Biome::BorealForest),
        5 | 14 => biome_color(Biome::Grassland),
        6 | 15 => biome_color(Biome::Desert),
        7 | 16 => biome_color(Biome::Tundra),
        8 | 17 => MOUNTAIN,
        _ => unreachable!("placeholder texture layer must be in the P2 terrain table"),
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

fn rgba8(color: LinearRgba) -> [u8; 4] {
    [
        f32_to_u8(color.red),
        f32_to_u8(color.green),
        f32_to_u8(color.blue),
        f32_to_u8(color.alpha),
    ]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn f32_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
