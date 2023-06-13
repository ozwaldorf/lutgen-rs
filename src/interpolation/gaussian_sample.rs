//! Gaussian sample based remapping. Samples a number of iterations of each pixel and
//! finds their nearest neighbors, averaging them all together for a final color.

use exoquant::{ColorSpace, Colorf};
use image::{Pixel, Rgb};
use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};

use super::{nearest_neighbor::NearestNeighborRemapper, InterpolatedRemapper};

/// Interpolated remap using a gaussian distribution set to sample and mix colors.
///
/// All combinations of a the pixel channels (up to sample_count^3) is computed,
/// remapped, and averaged together to get an interpolated color.
pub struct GaussianSamplingRemapper<'a, CS: ColorSpace + Sync> {
    iterations: usize,
    seed: u64,
    normal: Normal<f64>,
    inner_remapper: NearestNeighborRemapper<'a, CS>,
}

pub struct GaussianSamplingParams<CS: ColorSpace> {
    pub mean: f64,
    pub std_dev: f64,
    pub iterations: usize,
    pub seed: u64,
    pub colorspace: CS,
}

impl<'a, CS: ColorSpace + Send + Sync> InterpolatedRemapper<'a>
    for GaussianSamplingRemapper<'a, CS>
{
    type Params = GaussianSamplingParams<CS>;

    #[inline(always)]
    fn new(palette: &'a [[u8; 3]], params: Self::Params) -> Self {
        let normal = Normal::new(params.mean, params.std_dev).unwrap();
        let inner_remapper = NearestNeighborRemapper::new(palette, params.colorspace);

        Self {
            iterations: params.iterations,
            seed: params.seed,
            normal,
            inner_remapper,
        }
    }

    fn remap_pixel(&self, pixel: &mut Rgb<u8>) {
        let mut mean_color = Colorf {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 255.0,
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(self.seed);
        for _ in 0..self.iterations {
            let mut pixel = pixel.clone();
            // apply gaussian noise to channels
            for c in pixel.channels_mut() {
                *c = (*c as f64 + self.normal.sample(&mut rng)).round() as u8
            }

            // find the nearest nearest_neighbor
            self.inner_remapper.remap_pixel(&mut pixel);

            // Incremental average
            let total = self.iterations as f64;
            mean_color.r += pixel.0[0] as f64 / total;
            mean_color.g += pixel.0[1] as f64 / total;
            mean_color.b += pixel.0[2] as f64 / total;
        }

        // Round off and set the final color
        *pixel = Rgb([
            mean_color.r.round() as u8,
            mean_color.g.round() as u8,
            mean_color.b.round() as u8,
        ]);
    }
}
