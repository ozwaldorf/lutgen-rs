//! Heavily optimized algorithm for interpolated remapping, using gaussian distribution sampling
//! and averaging on each pixel.
//!
//! 1. Generate a distribution sample set.
//! 2. For each pixel:
//!     * Generate all combinations of the sample set added to each channel.
//!     * Remap each variant using `nearest_neighbor` lookups.
//!     * Average the variants together for the final interpolated color.

use exoquant::{Color, ColorSpace, Colorf};
use image::Rgb;
use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};

use super::{nearest_neighbor::NearestNeighborRemapper, InterpolatedRemapper};

/// Interpolated remap using a gaussian distribution set to sample and mix colors.
///
/// All combinations of a the pixel channels (up to sample_count^3) is computed,
/// remapped, and averaged together to get an interpolated color.
pub struct GaussianV1Remapper<'a, CS: ColorSpace + Sync> {
    distribution_samples: Vec<f64>,
    inner_remapper: NearestNeighborRemapper<'a, CS>,
}

pub struct GaussianV1Params<CS: ColorSpace> {
    pub mean: f64,
    pub std_dev: f64,
    pub iterations: usize,
    pub seed: u64,
    pub colorspace: CS,
}

impl<'a, CS: ColorSpace + Send + Sync> InterpolatedRemapper<'a> for GaussianV1Remapper<'a, CS> {
    type Params = GaussianV1Params<CS>;

    fn new(palette: &'a [Color], params: Self::Params) -> Self {
        // Build distribution values
        let sample_count = (params.iterations as f64).cbrt().round() as usize;
        let distribution_samples =
            gaussian_distribution(params.mean, params.std_dev, sample_count, params.seed);

        let inner_remapper = NearestNeighborRemapper::new(palette, params.colorspace);

        Self {
            distribution_samples,
            inner_remapper,
        }
    }

    fn remap_pixel(&self, pixel: &mut Rgb<u8>) {
        let len = self.distribution_samples.len();
        let total = (len * len * len) as f64;
        let mut mean_color = Colorf {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 255.0,
        };

        // Iterate over every combination of the color for the distribution set. loops n^3 times
        for r_delta in &self.distribution_samples {
            for g_delta in &self.distribution_samples {
                for b_delta in &self.distribution_samples {
                    // compute the guass color
                    let mut pixel = Rgb([
                        (pixel.0[0] as f64 + *r_delta) as u8,
                        (pixel.0[1] as f64 + *g_delta) as u8,
                        (pixel.0[2] as f64 + *b_delta) as u8,
                    ]);

                    // find the nearest nearest_neighbor
                    self.inner_remapper.remap_pixel(&mut pixel);

                    // Incremental average
                    mean_color.r += pixel.0[0] as f64 / total;
                    mean_color.g += pixel.0[1] as f64 / total;
                    mean_color.b += pixel.0[2] as f64 / total;
                }
            }
        }

        // Round off and set the final color
        *pixel = Rgb([
            mean_color.r.round() as u8,
            mean_color.g.round() as u8,
            mean_color.b.round() as u8,
        ]);
    }
}

/// Generate `n` values from a gaussian distribution curve.
pub fn gaussian_distribution(mean: f64, std_dev: f64, n: usize, seed: u64) -> Vec<f64> {
    let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut values = Vec::with_capacity(n);

    for _ in 0..n {
        let value = normal.sample(&mut rng);
        values.push(value);
    }

    values
}
