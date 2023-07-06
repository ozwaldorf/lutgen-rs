//! Gaussian sample based remapping. Samples a number of iterations of each pixel and
//! finds their nearest neighbors, averaging them all together for a final color.

use image::{Pixel, Rgb};
use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};

use super::{nearest_neighbor::NearestNeighborRemapper, InterpolatedRemapper};
use crate::GenerateLut;

/// Interpolated remapper using a gaussian distribution set to sample and mix colors.
/// Slow, compared to the RBF algorithms.
///
/// All combinations of a the pixel channels (up to sample_count^3) is computed,
/// remapped to the nearest neighbor, and averaged together to get an interpolated color.
pub struct GaussianSamplingRemapper<'a> {
    iterations: usize,
    seed: u64,
    normal: Normal<f64>,
    nearest_neighbor: NearestNeighborRemapper<'a>,
}

impl<'a> GaussianSamplingRemapper<'a> {
    #[inline(always)]
    pub fn new(
        palette: &'a [[u8; 3]],
        mean: f64,
        std_dev: f64,
        iterations: usize,
        lum_factor: f64,
        seed: u64,
    ) -> Self {
        let normal = Normal::new(mean, std_dev).unwrap();
        let nearest_neighbor = NearestNeighborRemapper::new(palette, lum_factor);

        Self {
            iterations,
            seed,
            normal,
            nearest_neighbor,
        }
    }
}

impl<'a> GenerateLut<'a> for GaussianSamplingRemapper<'a> {}
impl<'a> InterpolatedRemapper<'a> for GaussianSamplingRemapper<'a> {
    fn remap_pixel(&self, pixel: &mut Rgb<u8>) {
        let mut mean = [0f64; 3];

        let mut rng: StdRng = SeedableRng::seed_from_u64(self.seed);
        for _ in 0..self.iterations {
            let mut pixel = *pixel;
            // apply gaussian noise to channels
            for c in pixel.channels_mut() {
                *c = (*c as f64 + self.normal.sample(&mut rng)).round() as u8
            }

            // find the nearest neighbor
            self.nearest_neighbor.remap_pixel(&mut pixel);

            // Incremental average
            let total = self.iterations as f64;
            mean[0] += pixel.0[0] as f64 / total;
            mean[1] += pixel.0[1] as f64 / total;
            mean[2] += pixel.0[2] as f64 / total;
        }

        // Round off and set the final color
        *pixel = Rgb([
            mean[0].round() as u8,
            mean[1].round() as u8,
            mean[2].round() as u8,
        ]);
    }
}
