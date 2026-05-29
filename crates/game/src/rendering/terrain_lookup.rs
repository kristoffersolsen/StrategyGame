//! Terrain lookup and texture-array data for map rendering.

use std::path::Path;

use bevy::{
    asset::RenderAssetUsages,
    color::{ColorToComponents, LinearRgba},
    image::{CompressedImageFormats, Image, ImageSampler, ImageType},
    render::render_resource::TextureFormat,
};
use coords::Coord;
use worldgen::WorldData;

pub(crate) const TEXTURE_SIZE: u32 = 64;
pub(crate) const TEXTURE_LAYER_COUNT: u32 = 18;
pub(crate) const P3_TEXTURE_SIZE: u32 = 256;
pub(crate) const P3_TEXTURE_LAYER_COUNT: u32 = 17;

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
            let biome_handle = world.biomes.biome.get(coord).copied().flatten();
            let biome_id = biome_handle.map(|h| world.biome_registry.id_of(h));
            let mut color = biome_id.map_or(hex_color(0x7a, 0x9a, 0x3a), biome_color);
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
            let biome_handle = world.biomes.biome.get(coord).copied().flatten();
            let grassland_handle = world.biome_registry.resolve("grassland");
            let effective_handle = biome_handle.or(grassland_handle);
            let biome_id = effective_handle.map_or("grassland", |h| world.biome_registry.id_of(h));
            let base = biome_texture_index(biome_id);

            if is_hilly(world, coord) {
                base + 9
            } else {
                base
            }
        }
    }
}

/// Returns the P3 authored texture-array layer index used for a tile top.
pub(crate) fn p3_texture_index(world: &WorldData, coord: Coord, mountain_level: u8) -> u32 {
    let height_level = *world
        .tectonics
        .height_level
        .get(coord)
        .expect("rendered coord must be in bounds");

    match height_level {
        0 => 0,
        1 => 1,
        level if level >= mountain_level => 8,
        _ => {
            let biome_handle = world.biomes.biome.get(coord).copied().flatten();
            let grassland_handle = world.biome_registry.resolve("grassland");
            let effective_handle = biome_handle.or(grassland_handle);
            let biome_id = effective_handle.map_or("grassland", |h| world.biome_registry.id_of(h));
            let base = biome_texture_index(biome_id);

            if is_hilly(world, coord) {
                base + 9
            } else {
                base
            }
        }
    }
}

fn biome_texture_index(id: &str) -> u32 {
    match id {
        "tropical_rainforest" => 2,
        "temperate_forest" => 3,
        "boreal_forest" => 4,
        "desert" => 6,
        "tundra" => 7,
        // "grassland" and all unknown biomes use grassland texture index
        _ => 5,
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

/// Builds the authored terrain texture array used by the P3 renderer.
pub(crate) fn p3_texture_array_data() -> Vec<u8> {
    let mut layers = vec![Vec::new(); usize::try_from(P3_TEXTURE_LAYER_COUNT).unwrap()];

    layers[0] = load_p3_png_layer("ocean");
    layers[1] = load_p3_png_layer("coast");
    layers[2] = load_p3_png_layer("tropical");
    layers[3] = load_p3_png_layer("temperate");
    layers[4] = load_p3_png_layer("boreal");
    layers[5] = load_p3_png_layer("grassland");
    layers[6] = load_p3_png_layer("desert");
    layers[7] = load_p3_png_layer("tundra");
    layers[8] = load_p3_png_layer("mountain");

    layers[9] = darkened_layer(&layers[0], 0.82);
    layers[10] = darkened_layer(&layers[1], 0.82);
    layers[11] = darkened_layer(&layers[2], 0.82);
    layers[12] = darkened_layer(&layers[3], 0.82);
    layers[13] = darkened_layer(&layers[4], 0.82);
    layers[14] = darkened_layer(&layers[5], 0.82);
    layers[15] = darkened_layer(&layers[6], 0.82);
    layers[16] = darkened_layer(&layers[7], 0.82);

    let layer_len = p3_layer_len();
    let mut data = Vec::with_capacity(
        layer_len * usize::try_from(P3_TEXTURE_LAYER_COUNT).expect("layer count fits in usize"),
    );
    for layer in layers {
        assert_eq!(
            layer.len(),
            layer_len,
            "P3 terrain texture layer must be {P3_TEXTURE_SIZE}x{P3_TEXTURE_SIZE} RGBA8",
        );
        data.extend_from_slice(&layer);
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

fn biome_color(id: &str) -> LinearRgba {
    match id {
        "tropical_rainforest" => hex_color(0x2d, 0x6a, 0x2d),
        "temperate_forest" => hex_color(0x4a, 0x7c, 0x4a),
        "boreal_forest" => hex_color(0x4a, 0x6b, 0x55),
        "desert" => hex_color(0xc8, 0xa9, 0x6e),
        "tundra" => hex_color(0x8a, 0x9a, 0x8a),
        // "grassland" and all unknown biomes use grassland color
        _ => hex_color(0x7a, 0x9a, 0x3a),
    }
}

fn texture_layer_color(layer: u32) -> LinearRgba {
    match layer {
        0 | 9 => OCEAN,
        1 | 10 => COAST,
        2 | 11 => biome_color("tropical_rainforest"),
        3 | 12 => biome_color("temperate_forest"),
        4 | 13 => biome_color("boreal_forest"),
        5 | 14 => biome_color("grassland"),
        6 | 15 => biome_color("desert"),
        7 | 16 => biome_color("tundra"),
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

fn load_p3_png_layer(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/worldgen/textures")
        .join(format!("{name}.png"));
    let path_label = path.display().to_string();
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read P3 terrain texture {path_label}: {error}"));
    let image = Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .unwrap_or_else(|error| panic!("failed to decode P3 terrain texture {path_label}: {error}"));

    assert_eq!(
        image.texture_descriptor.size.width, P3_TEXTURE_SIZE,
        "P3 terrain texture {path_label} must be {P3_TEXTURE_SIZE}px wide",
    );
    assert_eq!(
        image.texture_descriptor.size.height, P3_TEXTURE_SIZE,
        "P3 terrain texture {path_label} must be {P3_TEXTURE_SIZE}px tall",
    );
    assert_eq!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb,
        "P3 terrain texture {path_label} must decode to Rgba8UnormSrgb",
    );

    image
        .data
        .expect("decoded P3 terrain texture must contain CPU pixel data")
}

fn darkened_layer(layer: &[u8], factor: f32) -> Vec<u8> {
    layer
        .chunks_exact(4)
        .flat_map(|pixel| {
            [
                f32_to_u8(f32::from(pixel[0]) / 255.0 * factor),
                f32_to_u8(f32::from(pixel[1]) / 255.0 * factor),
                f32_to_u8(f32::from(pixel[2]) / 255.0 * factor),
                pixel[3],
            ]
        })
        .collect()
}

fn p3_layer_len() -> usize {
    usize::try_from(P3_TEXTURE_SIZE * P3_TEXTURE_SIZE * 4)
        .expect("P3 texture layer size fits in usize")
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn f32_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
