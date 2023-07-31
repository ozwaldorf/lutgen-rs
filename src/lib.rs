#![doc = include_str!("../README.md")]

use image::{ImageBuffer, Rgb};
use interpolation::InterpolatedRemapper;

pub mod identity;
pub mod interpolation;

/// Core image type (Rgb8)
pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

pub trait GenerateLut<'a>: InterpolatedRemapper<'a> {
    /// Helper method to generate a lut using an [`InterpolatedRemapper`].
    fn generate_lut(&self, level: u8) -> Image {
        let mut identity = identity::generate(level);
        self.remap_image(&mut identity);
        identity
    }
}
