//! Interpolated remapping algorithms.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use gaussian_blur::GaussianBlurRemapper;
pub use gaussian_sample::GaussianSamplingRemapper;
use image::Rgba;
use kiddo::float::kdtree::KdTree;
pub use nearest_neighbor::NearestNeighborRemapper;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
pub use rbf::{GaussianRemapper, LinearRemapper, ShepardRemapper};

use crate::RgbaImage;

mod gaussian_blur;
mod gaussian_sample;
mod nearest_neighbor;
mod rbf;

/// Interpolated Remapper. Implements an algorithm with some initialization parameters.
pub trait InterpolatedRemapper<'a>: Sync {
    /// Remap a single pixel in place
    fn remap_pixel(&self, pixel: &mut Rgba<u8>);

    /// Remap an image in place. Default implementation uses `rayon` to iterate in parallel over
    /// the pixels.
    fn remap_image(&self, image: &mut RgbaImage) {
        image.pixels_mut().for_each(|pixel| {
            self.remap_pixel(pixel);
        });
    }

    /// Remap an image in place, aborting if the given atomic boolean is true.
    fn remap_image_with_interrupt(&self, image: &mut RgbaImage, abort: Arc<AtomicBool>) {
        image.pixels_mut().for_each(|pixel| {
            if !abort.load(std::sync::atomic::Ordering::Relaxed) {
                self.remap_pixel(pixel);
            }
        });
    }

    /// Rayon version
    #[cfg(feature = "rayon")]
    fn par_remap_image(&self, image: &mut RgbaImage) {
        image.par_pixels_mut().for_each(|pixel| {
            self.remap_pixel(pixel);
        });
    }

    /// Rayon version
    #[cfg(feature = "rayon")]
    fn par_remap_image_with_interrupt(&self, image: &mut RgbaImage, abort: Arc<AtomicBool>) {
        image.par_pixels_mut().for_each(|pixel| {
            if !abort.load(std::sync::atomic::Ordering::Relaxed) {
                self.remap_pixel(pixel);
            }
        });
    }
}

/// Type alias for our internal color tree for NN lookups
type ColorTree = KdTree<f64, u32, 3, 4, u32>;
