//! Interpolated remapping algorithms.

pub use gaussian_v0::*;
pub use gaussian_v1::*;
use image::Rgb;
pub use nearest_neighbor::*;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};

use crate::Image;

pub mod gaussian_v0;
pub mod gaussian_v1;
pub mod nearest_neighbor;

/// Interpolated Remapper. Implements an algorithm with some initialization parameters.
pub trait InterpolatedRemapper<'a>: Sync {
    /// Parameter for the algorithm
    type Params;

    fn new(palette: &'a [[u8; 3]], params: Self::Params) -> Self;

    /// Remap a single pixel in place
    fn remap_pixel(&self, pixel: &mut Rgb<u8>);

    /// Remap an image in place. Default implementation uses `rayon` to iterate in parallel over
    /// the pixels.
    fn remap_image(&self, image: &mut Image) {
        image
            .pixels_mut()
            .collect::<Vec<_>>()
            .par_iter_mut()
            .for_each(|pixel| {
                self.remap_pixel(pixel);
            });
    }
}
