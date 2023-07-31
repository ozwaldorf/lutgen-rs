#![doc = include_str!("../README.md")]

use image::{ImageBuffer, Pixel, Rgb};
use interpolation::InterpolatedRemapper;

pub mod identity;
pub mod interpolation;

/// Core image type (Rgb8)
pub type Image<P> = ImageBuffer<P, Vec<<P as Pixel>::Subpixel>>;
pub type LutImage = Image<Rgb<u8>>;

pub trait GenerateLut<'a>: InterpolatedRemapper<'a> {
    /// Helper method to generate a lut using an [`InterpolatedRemapper`].
    fn generate_lut(&self, level: u8) -> LutImage {
        let mut identity = identity::generate(level);
        self.remap_image(&mut identity);
        identity
    }
}
