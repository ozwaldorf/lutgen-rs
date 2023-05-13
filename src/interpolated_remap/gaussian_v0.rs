//! The OG way of doing interpolated palette remapping. Based off of the imagemagick solution:
//!
//! ```text
//! convert HALD:8 \
//!     -duplicate 512 \
//!     -attenuate 1 +noise Gaussian \
//!     -quantize LAB +dither -remap <palette.png> \
//!     -evaluate-sequence Mean <output.png>
//! ```

use exoquant::{Color, ColorSpace};
use image::{ImageBuffer, Pixel, Rgb};
use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};

use super::{nearest_neighbor::NearestNeighborRemapper, InterpolatedRemapper};
use crate::Image;

/// Remap an image into a guassian noise interpolated palette of colors. Much slower than v1.
///
/// Runs multiple iterations of random gaussian noise, remapping each variant, and finally computing
/// the mean of the sequence of images. The resulting image is an interpolated version of the palette, only
/// using combinations of the original palette colors.
pub struct GaussianV0Remapper<'a, CS: ColorSpace + Sync> {
    mean: f64,
    std_dev: f64,
    iterations: usize,
    seed: u64,
    inner_remapper: NearestNeighborRemapper<'a, CS>,
}

pub struct GaussianV0Params<CS: ColorSpace> {
    pub mean: f64,
    pub std_dev: f64,
    pub iterations: usize,
    pub seed: u64,
    pub colorspace: CS,
}

impl<'a, CS: ColorSpace + Send + Sync> InterpolatedRemapper<'a> for GaussianV0Remapper<'a, CS> {
    type Params = GaussianV0Params<CS>;

    fn new(palette: &'a [Color], params: Self::Params) -> Self {
        let Self::Params {
            mean,
            std_dev,
            iterations,
            seed,
            colorspace,
        } = params;

        let inner_remapper = NearestNeighborRemapper::new(palette, colorspace);

        Self {
            mean,
            std_dev,
            iterations,
            seed,
            inner_remapper,
        }
    }

    fn remap_image(&self, image: &mut Image) {
        rayon::scope(|s| {
            let (tx, rx) = std::sync::mpsc::channel();
            // Spawn a thread for each iteration
            for i in 0..self.iterations as u64 {
                let tx = tx.clone();
                let mut image = image.clone();
                s.spawn(move |_| {
                    // Apply noise to the image
                    gaussian_noise(&mut image, self.mean, self.std_dev, self.seed + i);
                    // Remap the variant
                    self.inner_remapper.remap_image(&mut image);
                    tx.send(image).unwrap();
                });
            }
            drop(tx);

            // Collect and evaluate the mean image
            *image = sequence_mean(rx.into_iter().collect());
        });
    }

    /// Always panics, use v1 algorithm for single pixel interpolation
    fn remap_pixel(&self, _: &mut Rgb<u8>) {
        unimplemented!("Use v1 for per pixel iterpolation")
    }
}

/// Adds independent additive Gaussian noise to all channels
/// of an image in place, with the given mean and standard deviation.
pub fn gaussian_noise(image: &mut Image, mean: f64, stddev: f64, seed: u64) {
    let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
    let normal = Normal::new(mean, stddev).unwrap();

    for p in image.pixels_mut() {
        for c in p.channels_mut() {
            let noise = normal.sample(&mut rng);
            *c = (*c as f64 + noise).round() as u8;
        }
    }
}

/// Evaluate a mean image for a sequence of images.
///
/// Undefined behavior when images aren't the same size.
pub fn sequence_mean(images: Vec<Image>) -> Image {
    let total = images.len() as f64;
    let first_image = &images[0];
    let (width, height) = first_image.dimensions();

    // Create a mutable buffer to store the sum of pixel values
    let mut buffer = vec![[0.0; 3]; (width * height) as usize];

    // Iterate over each pixel in all images and build a buffer of f64 integers
    for image in images {
        for (x, y, pixel) in image.enumerate_pixels() {
            let p = (y * width + x) as usize;

            buffer[p][0] += pixel.0[0] as f64 / total;
            buffer[p][1] += pixel.0[1] as f64 / total;
            buffer[p][2] += pixel.0[2] as f64 / total;
        }
    }

    // Write the combined pixels to a final image
    let mut output = ImageBuffer::new(width, height);
    for (x, y, pixel) in output.enumerate_pixels_mut() {
        let color = buffer[(y * width + x) as usize];
        *pixel = Rgb([color[0] as u8, color[1] as u8, color[2] as u8]);
    }

    output
}
