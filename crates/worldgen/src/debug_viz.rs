//! PNG debug visualization for generation fields.
//!
//! Provides two rendering modes for any [`Field<T>`]:
//!
//! - **Pixel mode** ([`dump_field_pixels`]) — one pixel per tile,
//!   row-major. Fast, tiny output (~36KB for a 240×150 map). Used by
//!   determinism tests that hash the image bytes and by quick sanity
//!   checks during development.
//!
//! - **Hex mode** ([`dump_field_hex`]) — each tile rendered as a
//!   rasterized pointy-top hexagon. Slow, large output (~20MB for a
//!   240×150 map at default size). Used for visual parameter tuning
//!   via `mapdump`.
//!
//! Both functions take a `to_f32` closure that converts the field's
//! element type to a normalized `f32` value, and a [`Colormap`] that
//! maps that value to an RGB triple. The caller controls the mapping;
//! the viz module controls the rendering.
//!
//! # Examples
//!
//! ```no_run
//! use std::path::Path;
//! use coords::MapBounds;
//! use worldgen::Field;
//! use worldgen::debug_viz::{dump_field_pixels, dump_field_hex, Viridis};
//!
//! let bounds = MapBounds::new(240, 150);
//! let elevation: Field<f32> = Field::filled(bounds, 0.5);
//!
//! // Fast pixel dump for tests
//! dump_field_pixels(&elevation, Path::new("/tmp/elev.png"), |v| v, &Viridis)
//!     .expect("write failed");
//!
//! // Hex dump for human inspection
//! dump_field_hex(&elevation, Path::new("/tmp/elev_hex.png"), |v| v, &Viridis, 8)
//!     .expect("write failed");
//! ```

use std::path::Path;

use coords::Coord;
use image::{ImageError, RgbImage};

use crate::Field;
use crate::world_data::{Biome, ResourceId};

// ------------- Error type -------------

/// Errors produced by the debug visualization functions.
#[derive(Debug, thiserror::Error)]
pub enum VisError {
    /// The output PNG could not be written to disk.
    #[error("failed to write PNG to {path}: {source}")]
    Io {
        /// The path that was being written.
        path: std::path::PathBuf,
        /// The underlying image library error.
        #[source]
        source: ImageError,
    },
}

// ------------- Colormap trait -------------

/// Maps a normalized scalar value to an RGB colour triple.
///
/// The input is a `f32` in `[0.0, 1.0]`. Inputs outside that range
/// are clamped by convention — implementations should not panic.
///
/// Callers supply a `to_f32` closure alongside the colormap to
/// convert the field's element type to the expected range.
pub trait Colormap {
    /// Returns an `[R, G, B]` triple for the given value.
    fn color(&self, value: f32) -> [u8; 3];
}

// ------------- Built-in colormaps -------------

/// Linear grayscale: `0.0` → black, `1.0` → white.
pub struct Grayscale;

impl Colormap for Grayscale {
    fn color(&self, value: f32) -> [u8; 3] {
        // clamp guarantees [0.0, 1.0]; * 255 + round gives [0.0, 255.0];
        // the f32 → u8 narrowing is intentional and lossless in this range.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        [v, v, v]
    }
}

/// Perceptually uniform Viridis colormap (256-entry LUT).
///
/// Viridis runs from dark purple (low) through green to yellow (high).
/// It is perceptually uniform and colorblind-safe, making it the
/// default choice for continuous scalar fields such as elevation,
/// temperature, and moisture.
///
/// The LUT is taken from the matplotlib Viridis definition.
pub struct Viridis;

impl Colormap for Viridis {
    fn color(&self, value: f32) -> [u8; 3] {
        // clamp + round gives [0.0, 255.0]; cast to usize is safe.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let idx = (value.clamp(0.0, 1.0) * 255.0).round() as usize;
        VIRIDIS_LUT[idx]
    }
}

/// 12-colour categorical palette for discrete fields such as biome
/// classifications or tectonic plate IDs.
///
/// Wraps around for indices above 11. Colours are chosen to be
/// visually distinct and distinguishable under common forms of
/// colour-blindness.
pub struct Categorical;

impl Colormap for Categorical {
    fn color(&self, value: f32) -> [u8; 3] {
        // abs() guarantees non-negative; cast to usize then mod is safe.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let idx = (value.abs() as usize) % CATEGORICAL_PALETTE.len();
        CATEGORICAL_PALETTE[idx]
    }
}

/// Categorical colormap for resource overlays.
pub struct ResourceCmap;

impl Colormap for ResourceCmap {
    fn color(&self, value: f32) -> [u8; 3] {
        match resource_value_to_id(value) {
            None => [18, 24, 28],
            Some(ResourceId::Iron) => [120, 120, 125],
            Some(ResourceId::Horses) => [185, 120, 55],
            Some(ResourceId::Gold) => [235, 190, 45],
            Some(ResourceId::Gems) => [60, 190, 210],
            Some(ResourceId::Spices) => [210, 80, 45],
            Some(ResourceId::Wine) => [145, 45, 115],
            Some(ResourceId::Wheat) => [225, 210, 105],
            Some(ResourceId::Fish) => [65, 145, 220],
            Some(ResourceId::Stone) => [150, 150, 135],
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn resource_value_to_id(value: f32) -> Option<ResourceId> {
    let index = value.round() as i32;
    match index {
        1 => Some(ResourceId::Iron),
        2 => Some(ResourceId::Horses),
        3 => Some(ResourceId::Gold),
        4 => Some(ResourceId::Gems),
        5 => Some(ResourceId::Spices),
        6 => Some(ResourceId::Wine),
        7 => Some(ResourceId::Wheat),
        8 => Some(ResourceId::Fish),
        9 => Some(ResourceId::Stone),
        _ => None,
    }
}

// ------------- Pixel-per-tile render (in-memory) -------------

/// Renders a one-pixel-per-tile [`RgbImage`] of `field`.
///
/// Identical logic to [`dump_field_pixels`] but returns the image
/// rather than writing it to disk. Used by `maptune` to display debug
/// figures inline without touching the filesystem.
///
/// `to_f32` must return values in `[0.0, 1.0]`; values outside that
/// range are clamped by the colormap.
pub fn render_field_pixels<T: Copy>(
    field: &Field<T>,
    to_f32: impl Fn(T) -> f32,
    colormap: &dyn Colormap,
) -> RgbImage {
    let bounds = field.bounds();
    let width = bounds.width();
    let height = bounds.height();
    let mut img = RgbImage::new(width, height);

    for (coord, &value) in field.iter_coords() {
        let px = coord.q.cast_unsigned();
        let py = coord.r.cast_unsigned();
        let rgb = colormap.color(to_f32(value));
        img.put_pixel(px, py, image::Rgb(rgb));
    }

    img
}

/// Renders a hex-rasterized [`RgbImage`] of `field`.
///
/// Identical logic to [`dump_field_hex`] but returns the image rather
/// than writing it to disk. Used by `maptune` to display debug figures
/// inline without touching the filesystem.
///
/// `to_f32` must return values in `[0.0, 1.0]`; values outside that
/// range are clamped by the colormap.
pub fn render_field_hex<T: Copy>(
    field: &Field<T>,
    to_f32: impl Fn(T) -> f32,
    colormap: &dyn Colormap,
    hex_size: u32,
) -> RgbImage {
    let bounds = field.bounds();
    let (img_width, img_height) = hex_image_dimensions(bounds.width(), bounds.height(), hex_size);
    let mut img = RgbImage::new(img_width, img_height);

    for px in img.pixels_mut() {
        *px = image::Rgb(HEX_BORDER_COLOR);
    }

    for (coord, &value) in field.iter_coords() {
        let rgb = colormap.color(to_f32(value));
        rasterize_hex(&mut img, coord, bounds.width(), hex_size, rgb);
    }

    img
}

/// Renders a hex-rasterized [`RgbImage`] from a pre-composed RGB field.
///
/// Identical logic to [`dump_rgb_field_hex`] but returns the image
/// rather than writing it to disk.
pub fn render_rgb_field_hex(field: &Field<[u8; 3]>, hex_size: u32) -> RgbImage {
    let bounds = field.bounds();
    let (img_width, img_height) = hex_image_dimensions(bounds.width(), bounds.height(), hex_size);
    let mut img = RgbImage::new(img_width, img_height);

    for px in img.pixels_mut() {
        *px = image::Rgb(HEX_BORDER_COLOR);
    }

    for (coord, &rgb) in field.iter_coords() {
        rasterize_hex(&mut img, coord, bounds.width(), hex_size, rgb);
    }

    img
}

// ------------- Pixel-per-tile dump -------------

/// Writes a one-pixel-per-tile PNG of `field` to `path`.
///
/// The output is `width × height` pixels in row-major order, matching
/// the storage layout of [`Field`]. This is the fast path: suitable
/// for determinism tests and quick sanity checks, but the hex
/// structure of the map is not visible in the output.
///
/// `to_f32` must return values in `[0.0, 1.0]`; values outside that
/// range are clamped by the colormap.
///
/// # Errors
///
/// Returns [`VisError::Io`] if the PNG cannot be written to `path`.
pub fn dump_field_pixels<T: Copy>(
    field: &Field<T>,
    path: &Path,
    to_f32: impl Fn(T) -> f32,
    colormap: &dyn Colormap,
) -> Result<(), VisError> {
    render_field_pixels(field, to_f32, colormap)
        .save(path)
        .map_err(|source| VisError::Io {
            path: path.to_owned(),
            source,
        })
}

// ------------- Hex-rasterized dump -------------

/// Writes a hex-rasterized PNG of `field` to `path`.
///
/// Each tile is rendered as a filled pointy-top hexagon with circumradius
/// `hex_size` pixels. The output accurately represents the hex grid
/// topology used by the `coords` crate, making it suitable for visual
/// parameter tuning.
///
/// For a 240×150 map with `hex_size = 8`, the output is approximately
/// 2880×2340 pixels (~20MB before compression). For `hex_size = 4`
/// the output is ~720×585 (~5MB). Larger hex sizes produce crisper
/// images at the cost of file size.
///
/// `to_f32` must return values in `[0.0, 1.0]`; values outside that
/// range are clamped by the colormap.
///
/// # Errors
///
/// Returns [`VisError::Io`] if the PNG cannot be written to `path`.
pub fn dump_field_hex<T: Copy>(
    field: &Field<T>,
    path: &Path,
    to_f32: impl Fn(T) -> f32,
    colormap: &dyn Colormap,
    hex_size: u32,
) -> Result<(), VisError> {
    render_field_hex(field, to_f32, colormap, hex_size)
        .save(path)
        .map_err(|source| VisError::Io {
            path: path.to_owned(),
            source,
        })
}

/// Writes a one-pixel-per-tile RGB PNG of `field` to `path`.
///
/// This is the direct-color companion to [`dump_field_pixels`], used
/// when callers need to compose multiple data layers before rendering
/// rather than mapping one scalar field through a colormap.
///
/// # Errors
///
/// Returns [`VisError::Io`] if the PNG cannot be written to `path`.
pub fn dump_rgb_field_pixels(field: &Field<[u8; 3]>, path: &Path) -> Result<(), VisError> {
    let bounds = field.bounds();
    let width = bounds.width();
    let height = bounds.height();
    let mut img = RgbImage::new(width, height);

    for (coord, &rgb) in field.iter_coords() {
        let px = coord.q.cast_unsigned();
        let py = coord.r.cast_unsigned();
        img.put_pixel(px, py, image::Rgb(rgb));
    }

    img.save(path).map_err(|source| VisError::Io {
        path: path.to_owned(),
        source,
    })
}

/// Writes a hex-rasterized RGB PNG of `field` to `path`.
///
/// This is the direct-color companion to [`dump_field_hex`], used for
/// composited overlay maps such as elevation plus rivers, lakes, and
/// canyons.
///
/// # Errors
///
/// Returns [`VisError::Io`] if the PNG cannot be written to `path`.
pub fn dump_rgb_field_hex(
    field: &Field<[u8; 3]>,
    path: &Path,
    hex_size: u32,
) -> Result<(), VisError> {
    render_rgb_field_hex(field, hex_size)
        .save(path)
        .map_err(|source| VisError::Io {
            path: path.to_owned(),
            source,
        })
}

/// Returns the pixel dimensions of a hex-rasterized image for a map
/// with the given tile dimensions and hex circumradius.
///
/// Exposed so callers can pre-allocate or log the output size before
/// writing.
#[must_use]
pub fn hex_image_dimensions(map_width: u32, map_height: u32, hex_size: u32) -> (u32, u32) {
    // Pointy-top axial hex geometry, wrapped horizontally to keep the
    // cylindrical map rectangular in debug output.
    let s = f64::from(hex_size);
    let inradius = s * SQRT3 / 2.0;
    let max_center_y = 1.5 * s * f64::from(map_height - 1);
    // ceil() produces a value that fits in u32 for any reasonable map size.
    // The truncation is intentional: we want the pixel ceiling, not a float.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let w = (SQRT3 * s * f64::from(map_width) + 2.0 * inradius + 2.0).ceil() as u32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let h = (max_center_y + 2.0 * s + 2.0).ceil() as u32;
    (w, h)
}

// ------------- Internal geometry helpers -------------

const SQRT3: f64 = 1.732_050_808_569_442;

/// Returns the pixel-space centre of the hex at `coord`.
///
/// Converts axial `(q, r)` to pointy-top pixel coordinates, wrapping
/// the horizontal coordinate into the map width:
///
/// ```text
/// x = √3 * size * wrap(q + r / 2, map_width)
/// y = 3/2 * size * r
/// ```
///
/// A raw axial rectangle projects to a parallelogram. Because this map
/// is cylindrical, the row shear can be absorbed into the east-west
/// wrap seam instead, preserving a rectangular PNG footprint.
///
/// We add padding of `size` on each axis so the topmost/leftmost
/// tile is not clipped by the image boundary.
fn hex_center(coord: Coord, map_width: u32, hex_size: u32) -> (f64, f64) {
    let size = f64::from(hex_size);
    let q = f64::from(coord.q);
    let r = f64::from(coord.r);
    let wrapped_x = wrap_unit(q + 0.5 * r, f64::from(map_width));
    let cx = size * SQRT3 * wrapped_x + size * SQRT3 / 2.0 + 1.0;
    let cy = size * 1.5 * r + size + 1.0;
    (cx, cy)
}

/// Wraps `value` into `[0, period)`.
fn wrap_unit(value: f64, period: f64) -> f64 {
    value.rem_euclid(period)
}

/// Background color painted behind all hex tiles.
///
/// Visible as the border between adjacent tiles. Dark grey reads as
/// clean grid lines against all colormap palettes.
const HEX_BORDER_COLOR: [u8; 3] = [40, 40, 40];

/// Width of the border between adjacent tiles, in pixels.
///
/// Each hex is filled with radius `circumradius - HEX_BORDER_WIDTH`,
/// leaving a gap of this many pixels around each tile edge. Set to
/// `0.0` to produce a borderless fill (faster but loses grid lines
/// and introduces boundary artifacts from floating-point edge cases).
const HEX_BORDER_WIDTH: f64 = 1.0;

/// Rasterizes a single pointy-top hexagon centred at `coord` into `img`.
///
/// Uses a bounding-box scan with a point-in-hexagon test. The
/// circumradius is `hex_size` pixels; the inradius (centre to edge
/// midpoint) is `hex_size * √3/2`. The filled area is inset by
/// [`HEX_BORDER_WIDTH`] pixels so adjacent tiles leave a visible gap
/// that renders as grid lines against the background color.
fn rasterize_hex(img: &mut RgbImage, coord: Coord, map_width: u32, hex_size: u32, rgb: [u8; 3]) {
    let size = f64::from(hex_size);
    // Inset the fill radius so tiles don't touch. This simultaneously
    // produces visible borders and eliminates boundary artifacts caused
    // by floating-point ambiguity at the exact edge between two tiles.
    let fill_size = size - HEX_BORDER_WIDTH;
    let (cx, cy) = hex_center(coord, map_width, hex_size);
    rasterize_hex_at_center(img, cx, cy, size, fill_size, rgb);

    // Tiles crossing the visual wrap seam need a duplicate on the
    // opposite edge so the rectangular PNG has no clipped seam gaps.
    let wrap_width_px = SQRT3 * size * f64::from(map_width);
    if cx - size < 0.0 {
        rasterize_hex_at_center(img, cx + wrap_width_px, cy, size, fill_size, rgb);
    }
    if cx + size >= f64::from(img.width()) {
        rasterize_hex_at_center(img, cx - wrap_width_px, cy, size, fill_size, rgb);
    }
}

fn rasterize_hex_at_center(
    img: &mut RgbImage,
    cx: f64,
    cy: f64,
    size: f64,
    fill_size: f64,
    rgb: [u8; 3],
) {
    // Bounding box: circumscribed rectangle of the full (unclipped) hex.
    // floor()/ceil() make the f64→i64 truncation exact (no fractional
    // part remains). .max(0) and .min(img_dim-1) guarantee the i64→u32
    // narrowing is lossless.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let x_min = ((cx - size).floor() as i64).max(0) as u32;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let x_max = ((cx + size).ceil() as i64).min(i64::from(img.width()) - 1) as u32;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let y_min = ((cy - size).floor() as i64).max(0) as u32;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let y_max = ((cy + size).ceil() as i64).min(i64::from(img.height()) - 1) as u32;

    for py in y_min..=y_max {
        for px in x_min..=x_max {
            if point_in_pointy_top_hex(f64::from(px), f64::from(py), cx, cy, fill_size) {
                img.put_pixel(px, py, image::Rgb(rgb));
            }
        }
    }
}

/// Returns `true` if pixel `(px, py)` lies inside a pointy-top hexagon
/// centred at `(cx, cy)` with circumradius `s`.
///
/// Uses three separating-axis conditions from the hex edge normals:
///
/// - Top/bottom vertices:           `|dy| ≤ s`
/// - Left/right edges:              `|dx| ≤ inradius` where `inradius = s * √3/2`
/// - Diagonal edges:                `|dy| + |dx| / √3 ≤ s`
///
/// All three must hold for the point to be inside. The derivation of
/// condition 3 comes from the diagonal edge passing through `(0, s)`
/// and `(inradius, s / 2)`.
fn point_in_pointy_top_hex(px: f64, py: f64, cx: f64, cy: f64, circumradius: f64) -> bool {
    let dx = (px - cx).abs();
    let dy = (py - cy).abs();
    let inr = circumradius * SQRT3 / 2.0;
    dy <= circumradius && dx <= inr && dy + dx / SQRT3 <= circumradius
}

// ------------- Viridis LUT -------------

// 256-entry Viridis LUT. Values derived from the matplotlib Viridis
// colormap definition (New BSD License). Each entry is [R, G, B]
// with components in [0, 255], converted from the authoritative
// float triples via round(v * 255).
#[rustfmt::skip]
const VIRIDIS_LUT: [[u8; 3]; 256] = [
    [68,1,84], [68,2,86], [69,4,87], [69,5,89],
    [70,7,90], [70,8,92], [70,10,93], [70,11,94],
    [71,13,96], [71,14,97], [71,16,99], [71,17,100],
    [71,19,101], [72,20,103], [72,22,104], [72,23,105],
    [72,24,106], [72,26,108], [72,27,109], [72,28,110],
    [72,29,111], [72,31,112], [72,32,113], [72,33,115],
    [72,35,116], [72,36,117], [72,37,118], [72,38,119],
    [72,40,120], [72,41,121], [71,42,122], [71,44,122],
    [71,45,123], [71,46,124], [71,47,125], [70,48,126],
    [70,50,126], [70,51,127], [70,52,128], [69,53,129],
    [69,55,129], [69,56,130], [68,57,131], [68,58,131],
    [68,59,132], [67,61,132], [67,62,133], [66,63,133],
    [66,64,134], [66,65,134], [65,66,135], [65,68,135],
    [64,69,136], [64,70,136], [63,71,136], [63,72,137],
    [62,73,137], [62,74,137], [62,76,137], [61,77,138],
    [61,78,138], [60,79,138], [60,80,138], [59,81,139],
    [59,82,139], [58,83,139], [58,84,139], [57,85,139],
    [57,86,139], [56,88,139], [56,89,139], [55,90,139],
    [55,91,139], [54,92,139], [54,93,139], [53,94,139],
    [53,95,139], [52,96,139], [52,97,139], [51,98,139],
    [51,99,139], [50,100,139], [50,101,138], [49,102,138],
    [49,103,138], [49,104,138], [48,105,138], [48,106,137],
    [47,107,137], [47,108,137], [46,109,137], [46,110,136],
    [46,111,136], [45,112,136], [45,113,136], [44,113,135],
    [44,114,135], [44,115,135], [43,116,134], [43,117,134],
    [42,118,133], [42,119,133], [42,120,133], [41,121,132],
    [41,122,132], [41,123,131], [40,124,131], [40,125,130],
    [39,126,130], [39,127,130], [39,128,129], [38,129,129],
    [38,130,128], [38,130,128], [37,131,127], [37,132,127],
    [37,133,126], [36,134,126], [36,135,125], [35,136,125],
    [35,137,124], [35,138,123], [34,139,123], [34,140,122],
    [34,141,122], [33,142,121], [33,143,121], [33,144,120],
    [33,145,119], [32,146,119], [32,146,118], [32,147,118],
    [31,148,117], [31,149,116], [31,150,116], [31,151,115],
    [31,152,115], [31,153,114], [31,154,113], [30,155,113],
    [30,156,112], [30,157,112], [31,158,111], [31,159,110],
    [31,160,110], [31,161,109], [31,161,108], [31,162,108],
    [32,163,107], [32,164,107], [33,165,106], [33,166,105],
    [34,167,105], [34,168,104], [35,169,104], [36,170,103],
    [37,171,102], [37,172,102], [38,173,101], [39,173,101],
    [40,174,100], [41,175,100], [42,176,99], [44,177,99],
    [45,178,98], [46,179,98], [47,180,97], [49,181,97],
    [50,182,96], [52,182,96], [53,183,95], [55,184,95],
    [56,185,94], [58,186,94], [59,187,94], [61,188,93],
    [63,188,93], [64,189,93], [66,190,92], [68,191,92],
    [70,192,92], [72,193,91], [74,193,91], [76,194,91],
    [78,195,91], [80,196,90], [82,197,90], [84,197,90],
    [86,198,90], [88,199,90], [90,200,90], [92,200,90],
    [94,201,89], [96,202,89], [99,203,89], [101,203,89],
    [103,204,89], [105,205,89], [108,205,89], [110,206,88],
    [112,207,88], [115,208,88], [117,208,87], [119,209,87],
    [122,209,87], [124,210,86], [127,211,86], [129,211,86],
    [132,212,85], [134,213,85], [137,213,84], [139,214,84],
    [142,214,84], [144,215,83], [147,215,83], [149,216,82],
    [152,216,82], [155,217,81], [157,217,81], [160,218,81],
    [162,218,80], [165,219,80], [168,219,79], [170,220,79],
    [173,220,78], [176,221,78], [178,221,77], [181,222,77],
    [184,222,76], [186,222,76], [189,223,75], [192,223,75],
    [194,223,74], [197,224,74], [200,224,73], [202,225,73],
    [205,225,72], [208,225,72], [210,226,71], [213,226,71],
    [216,226,70], [218,227,70], [221,227,69], [223,227,69],
    [226,228,69], [229,228,68], [231,228,68], [234,229,67],
    [236,229,67], [239,229,66], [241,229,66], [244,230,66],
    [246,230,65], [248,230,65], [251,231,65], [253,231,64],
];

// ------------- Categorical palette -------------

// 12 visually distinct colours, accessible under common forms of
// colour-blindness. Based on the ColorBrewer Set3 palette.
#[rustfmt::skip]
const CATEGORICAL_PALETTE: [[u8; 3]; 12] = [
    [141, 211, 199],
    [255, 255, 179],
    [190, 186, 218],
    [251, 128, 114],
    [128, 177, 211],
    [253, 180,  98],
    [179, 222, 105],
    [252, 205, 229],
    [217, 217, 217],
    [188, 128, 189],
    [204, 235, 197],
    [255, 237, 111],
];

// ------------- Domain-specific colormaps -------------

/// Four-colour map for tectonic boundary types (step 5).
///
/// Maps the encoded float produced by the boundary-type `to_f32` closure:
/// - `0.0` (interior) → near-black
/// - `1.0` (transform) → mid-grey
/// - `2.0` (divergent) → blue
/// - `3.0` (convergent) → red
pub struct BoundaryCmap;

impl Colormap for BoundaryCmap {
    fn color(&self, value: f32) -> [u8; 3] {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        match (value.round() as u8).min(3) {
            0 => [30, 30, 30],    // interior  — near-black
            1 => [140, 140, 140], // transform — grey
            2 => [60, 120, 220],  // divergent — blue
            _ => [220, 60, 60],   // convergent — red
        }
    }
}

/// Colour map for the discrete height levels produced by step 8.
///
/// Ocean and coast are fixed colours; land levels are linearly
/// interpolated from orange to red based on elevation; mountain is a
/// fixed deep red. The number of land levels is taken from
/// [`Self::land_level_count`], matching [`crate::params::TectonicsParams::land_level_count`].
///
/// Fixed anchors:
/// - `0` ocean    → dark blue  `[15, 55, 120]`
/// - `1` coast    → light blue `[80, 155, 215]`
/// - last level   → mountain   `[130, 15, 15]`
///
/// Land levels `2..=land_level_count+1` interpolate orange `[230, 155, 55]`
/// → red `[175, 40, 20]`.
pub struct HeightLevelCmap {
    /// Number of discrete land levels, matching [`crate::params::TectonicsParams::land_level_count`].
    pub land_level_count: u8,
}

impl Colormap for HeightLevelCmap {
    fn color(&self, value: f32) -> [u8; 3] {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let level = value.round() as u8;
        let mountain = self.land_level_count + 2;
        match level {
            0 => [15, 55, 120],
            1 => [80, 155, 215],
            l if l >= mountain => [130, 15, 15],
            l => {
                let land_idx = l - 2;
                let span = f32::from((self.land_level_count - 1).max(1));
                #[allow(clippy::cast_precision_loss)]
                let t = f32::from(land_idx) / span;
                let lerp = |a: u8, b: u8| -> u8 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        (f32::from(a) + t * (f32::from(b) - f32::from(a))).round() as u8
                    }
                };
                [lerp(230, 175), lerp(155, 40), lerp(55, 20)]
            }
        }
    }
}

/// Categorical colour map for biome classifications.
pub struct BiomeCmap;

impl BiomeCmap {
    /// Returns the RGB colour for an optional biome.
    #[must_use]
    pub const fn color_for_biome(biome: Option<Biome>) -> [u8; 3] {
        match biome {
            None => [45, 55, 65],
            Some(Biome::TropicalRainforest) => [0, 118, 74],
            Some(Biome::TemperateForest) => [80, 145, 60],
            Some(Biome::BorealForest) => [43, 92, 105],
            Some(Biome::Grassland) => [196, 190, 85],
            Some(Biome::Desert) => [218, 184, 105],
            Some(Biome::Tundra) => [198, 212, 218],
        }
    }
}

impl Colormap for BiomeCmap {
    fn color(&self, value: f32) -> [u8; 3] {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        match value.round() as u8 {
            1 => Self::color_for_biome(Some(Biome::TropicalRainforest)),
            2 => Self::color_for_biome(Some(Biome::TemperateForest)),
            3 => Self::color_for_biome(Some(Biome::BorealForest)),
            4 => Self::color_for_biome(Some(Biome::Grassland)),
            5 => Self::color_for_biome(Some(Biome::Desert)),
            6 => Self::color_for_biome(Some(Biome::Tundra)),
            _ => Self::color_for_biome(None),
        }
    }
}

// ------------- Seed overlay -------------

/// Writes a pixel-per-tile PNG of `base_field` with `seeds` painted
/// over it as single-pixel markers in `marker_color`.
///
/// The output is identical to [`dump_field_pixels`] except that the
/// tiles at each seed coordinate are overwritten with `marker_color`,
/// making seed positions visible against the base field.
///
/// Useful for step-1 verification: render a blank or gradient base
/// field, overlay the Poisson-disk seeds, and confirm that they are
/// evenly spaced without obvious clumping or seam artifacts.
///
/// `to_f32` must return values in `[0.0, 1.0]`; values outside that
/// range are clamped by the colormap.
///
/// # Errors
///
/// Returns [`VisError::Io`] if the PNG cannot be written to `path`.
pub fn dump_field_with_seed_overlay<T, F, C>(
    field: &Field<T>,
    seeds: &[coords::Coord],
    path: &Path,
    to_f32: F,
    colormap: &C,
    marker_color: [u8; 3],
) -> Result<(), VisError>
where
    F: Fn(&T) -> f32,
    C: Colormap,
{
    let bounds = field.bounds();
    let width = bounds.width();
    let height = bounds.height();

    let mut img = RgbImage::new(width, height);

    // Render base field.
    for (coord, value) in field.iter_coords() {
        let pixel = colormap.color(to_f32(value));
        // Coords are in-bounds by construction; the cast is safe.
        #[allow(clippy::cast_sign_loss)]
        img.put_pixel(coord.q as u32, coord.r as u32, image::Rgb(pixel));
    }

    // Paint seed markers on top.
    for &seed in seeds {
        if let Some(normalized) = bounds.normalize(seed) {
            #[allow(clippy::cast_sign_loss)]
            img.put_pixel(
                normalized.q as u32,
                normalized.r as u32,
                image::Rgb(marker_color),
            );
        }
    }

    img.save(path).map_err(|source| VisError::Io {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use coords::MapBounds;

    // ------------- Colormap -------------

    #[test]
    fn grayscale_zero_is_black() {
        assert_eq!(Grayscale.color(0.0), [0, 0, 0]);
    }

    #[test]
    fn grayscale_one_is_white() {
        assert_eq!(Grayscale.color(1.0), [255, 255, 255]);
    }

    #[test]
    fn grayscale_clamps_above_one() {
        assert_eq!(Grayscale.color(2.0), [255, 255, 255]);
    }

    #[test]
    fn grayscale_clamps_below_zero() {
        assert_eq!(Grayscale.color(-1.0), [0, 0, 0]);
    }

    #[test]
    fn viridis_endpoints_are_purple_and_yellow() {
        // Low values should be dark purple, high values yellow-ish.
        let low = Viridis.color(0.0);
        let high = Viridis.color(1.0);
        // Low: R < G, R < B (purple)
        assert!(low[0] < low[2], "low end should be purple-ish, got {low:?}");
        // High: R > G, R > B (yellow)
        assert!(
            high[0] > high[2],
            "high end should be yellow-ish, got {high:?}"
        );
    }

    #[test]
    fn categorical_wraps_for_large_index() {
        let a = Categorical.color(0.0);
        let b = Categorical.color(12.0);
        assert_eq!(a, b);
    }

    // ------------- hex_image_dimensions -------------

    #[test]
    fn hex_dimensions_are_positive() {
        let (w, h) = hex_image_dimensions(240, 150, 8);
        assert!(w > 0);
        assert!(h > 0);
    }

    #[test]
    fn hex_dimensions_scale_with_hex_size() {
        let (w1, h1) = hex_image_dimensions(10, 10, 4);
        let (w2, h2) = hex_image_dimensions(10, 10, 8);
        assert!(w2 > w1);
        assert!(h2 > h1);
    }

    #[test]
    fn hex_dimensions_keep_rectangular_map_footprint() {
        let (w, h) = hex_image_dimensions(120, 80, 4);
        let aspect = f64::from(w) / f64::from(h);
        let map_aspect = 120.0 / 80.0;
        assert!(
            (aspect - map_aspect).abs() < 1.0,
            "hex render should not shear the map into a wide parallelogram: {aspect}"
        );
    }

    // ------------- point_in_pointy_top_hex -------------

    #[test]
    fn center_is_inside_hex() {
        assert!(point_in_pointy_top_hex(0.0, 0.0, 0.0, 0.0, 8.0));
    }

    #[test]
    fn far_corner_is_outside_hex() {
        // A point beyond the circumradius on the vertical axis.
        assert!(!point_in_pointy_top_hex(0.0, 8.1, 0.0, 0.0, 8.0));
    }

    // ------------- dump_field_pixels -------------

    #[test]
    fn pixel_dump_produces_correct_dimensions() {
        let bounds = MapBounds::new(20, 10);
        let field = Field::filled(bounds, 0.5_f32);
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.png");
        dump_field_pixels(&field, &path, |v| v, &Grayscale).expect("dump failed");

        let img = image::open(&path).expect("open").into_rgb8();
        assert_eq!(img.width(), 20);
        assert_eq!(img.height(), 10);
    }

    #[test]
    fn pixel_dump_uniform_field_is_uniform_color() {
        let bounds = MapBounds::new(10, 10);
        let field = Field::filled(bounds, 1.0_f32);
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.png");
        dump_field_pixels(&field, &path, |v| v, &Grayscale).expect("dump failed");

        let img = image::open(&path).expect("open").into_rgb8();
        for px in img.pixels() {
            assert_eq!(px.0, [255, 255, 255]);
        }
    }

    #[test]
    fn hex_dump_produces_larger_image_than_pixel_dump() {
        let bounds = MapBounds::new(20, 10);
        let field = Field::filled(bounds, 0.5_f32);
        let dir = tempfile::tempdir().expect("tempdir");

        let px_path = dir.path().join("px.png");
        let hex_path = dir.path().join("hex.png");

        dump_field_pixels(&field, &px_path, |v| v, &Grayscale).expect("pixel dump");
        dump_field_hex(&field, &hex_path, |v| v, &Grayscale, 8).expect("hex dump");

        let px_img = image::open(&px_path).expect("open px").into_rgb8();
        let hex_img = image::open(&hex_path).expect("open hex").into_rgb8();

        assert!(hex_img.width() > px_img.width());
        assert!(hex_img.height() > px_img.height());
    }
}
