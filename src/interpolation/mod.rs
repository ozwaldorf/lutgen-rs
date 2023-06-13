//! Interpolated remapping algorithms.

pub use gaussian_sample::GaussianSamplingRemapper;
use image::Rgb;
pub use nearest_neighbor::NearestNeighborRemapper;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
pub use rbf::{gaussian::GaussianRemapper, linear::LinearRemapper, shepard::ShepardRemapper};

use crate::Image;

mod gaussian_sample;
mod nearest_neighbor;
mod rbf;

/// Interpolated Remapper. Implements an algorithm with some initialization parameters.
pub trait InterpolatedRemapper<'a>: Sync {
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
