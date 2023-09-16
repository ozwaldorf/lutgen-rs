//! Interpolated remapping algorithms.

pub use gaussian_sample::GaussianSamplingRemapper;
use image::Rgb;
use kiddo::float::kdtree::KdTree;
pub use nearest_neighbor::NearestNeighborRemapper;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
pub use rbf::{GaussianRemapper, LinearRemapper, ShepardRemapper};

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

type ColorTree = KdTree<f64, u16, 3, 4, u16>;
fn squared_euclidean(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dl = (a[0] - b[0]).powi(2);
    let da = (a[1] - b[1]).powi(2);
    let db = (a[2] - b[2]).powi(2);
    dl + da + db
}
