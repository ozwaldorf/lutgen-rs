#![doc = include_str!("../README.md")]

use image::{ImageBuffer, Rgb};
use interpolated_remap::InterpolatedRemapper;
/// Predefined constant palettes for popular colorschemes.
#[cfg(feature = "palettes")]
pub use lutgen_palettes::Palette;

pub mod identity;
pub mod interpolated_remap;

/// Core image type (Rgb8)
pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

/// Generic method to generate a lut using anything that implements [`InterpolatedRemapper`]
pub fn generate_lut<'a, A: InterpolatedRemapper<'a>>(
    level: u32,
    palette: &'a [[u8; 3]],
    params: A::Params,
) -> Image {
    let remapper = A::new(palette, params);
    let mut identity = identity::generate(level);
    remapper.remap_image(&mut identity);
    identity
}
