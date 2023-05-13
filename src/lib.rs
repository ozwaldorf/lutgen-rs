#![doc = include_str!("../README.md")]

use exoquant::{Color, SimpleColorSpace};
use image::{ImageBuffer, Rgb};
use interpolated_remap::{
    GaussianV0Params, GaussianV0Remapper, GaussianV1Params, GaussianV1Remapper,
    InterpolatedRemapper,
};
/// Palettes for popular colorschemes
#[cfg(feature = "palettes")]
pub use lutgen_palettes::Palette;

pub mod identity;
pub mod interpolated_remap;

/// Core image type (Rgb8)
pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

/// Generic generate method using any algorithm that implements
pub fn generate_lut<'a, A: InterpolatedRemapper<'a>>(
    level: u32,
    palette: &'a [Color],
    params: A::Params,
) -> Image {
    let remapper = A::new(palette, params);
    let mut identity = identity::generate(level);
    remapper.remap_image(&mut identity);
    identity
}

/// Helper method for generating a v0 gaussian interpolated hald-clut from a few parameters.
pub fn generate_simple_gaussian_v0_lut(
    palette: &[Color],
    level: u32,
    mean: f64,
    std_dev: f64,
    iterations: usize,
    seed: u64,
) -> Image {
    let colorspace = SimpleColorSpace::default();
    let params = GaussianV0Params {
        mean,
        std_dev,
        iterations,
        seed,
        colorspace,
    };
    generate_lut::<GaussianV0Remapper<_>>(level, palette, params)
}

/// Helper method for generating a v1 gaussian interpolated hald-clut from a few parameters.
pub fn generate_simple_gaussian_v1_lut(
    palette: &[Color],
    level: u32,
    mean: f64,
    std_dev: f64,
    iterations: usize,
    seed: u64,
) -> Image {
    let colorspace = SimpleColorSpace::default();
    let params = GaussianV1Params {
        mean,
        std_dev,
        iterations,
        seed,
        colorspace,
    };
    generate_lut::<GaussianV1Remapper<_>>(level, palette, params)
}
