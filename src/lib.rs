#![doc = include_str!("../README.md")]

use exoquant::Color;
use image::{ImageBuffer, Rgb};

pub mod identity;
pub mod interpolated_remap;

/// Core image type (Rgb8)
pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

/// Helper method for generating a v0 interpolated hald-clut from a few parameters.
pub fn generate_v0_lut(
    palette: &[Color],
    level: u32,
    mean: f64,
    std_dev: f64,
    iterations: usize,
) -> Image {
    let identity = identity::generate(level);
    interpolated_remap::v0::remap_image(&identity, palette, mean, std_dev, iterations)
}

/// Helper method for generating a v1 interpolated hald-clut from a few parameters.
pub fn generate_v1_lut(
    palette: &[Color],
    level: u32,
    mean: f64,
    std_dev: f64,
    iterations: usize,
) -> Image {
    let identity = identity::generate(level);
    interpolated_remap::v1::remap_image(identity, palette, mean, std_dev, iterations)
}
