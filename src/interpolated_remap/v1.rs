//! Heavily optimized algorithm for interpolated remapping, using gaussian distribution sampling
//! and averaging on each pixel.
//!
//! 1. Generate a distribution sample set.
//! 2. For each pixel:
//!     * Generate all combinations of the sample set added to each channel.
//!     * Remap each variant using `nearest_neighbor` lookups.
//!     * Average the variants together for the final interpolated color.

use exoquant::{Color, ColorMap, ColorSpace, Colorf, SimpleColorSpace};
use image::Rgb;
use rand::{rngs::StdRng, SeedableRng};
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;

use crate::Image;

/// Pixel sampling based interpolated palette remapping.
///
/// All combinations of a sampled gaussian distribution (up to sample_count) is applied to each pixel,
/// remapped, and averaged together to get an interpolated color.
///
/// * `image`: Image to remap
/// * `palette`: Base palette of colors to interpolate between.
/// * `mean`: Mean to use for the gaussian distribution.
/// * `std_dev`: Standard deviation to use for the gaussian distribution.
/// * `iterations`: Number of target iterations for each color. The cube root of this is used to
///                 determine the number of samples from the gaussian distribution curve.
pub fn remap_image(
    mut image: Image,
    palette: &[Color],
    mean: f64,
    std_dev: f64,
    iterations: usize,
    seed: u64,
) -> Image {
    // Build distribution values
    let sample_count = (iterations as f64).cbrt().round() as usize;
    let distribution_samples = gaussian_distribution(mean, std_dev, sample_count, seed);

    // Setup the colorspace and map
    let colorspace = SimpleColorSpace::default();
    let color_map =
        ColorMap::from_float_colors(palette.iter().map(|c| colorspace.to_float(*c)).collect());

    image
        .pixels_mut()
        .collect::<Vec<_>>()
        .par_iter_mut()
        .for_each(|pixel| {
            let color = remap_pixel_with_gaussian_distribution(
                &pixel.0,
                palette,
                &color_map,
                &colorspace,
                &distribution_samples,
            );

            **pixel = Rgb(color);
        });

    image
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

/// Remap a pixel using gaussian distribution.
///
/// Creates `distribution.len()^3` variants, remaps them against a palette, and averages the
/// results together to create an interpolated palette color.
pub fn remap_pixel_with_gaussian_distribution<CS: ColorSpace>(
    color: &[u8; 3],
    palette: &[Color],
    color_map: &ColorMap,
    colorspace: &CS,
    distribution: &[f64],
) -> [u8; 3] {
    let len = distribution.len();
    let total = (len * len * len) as f64;
    let mut mean_color = Colorf {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 255.0,
    };

    // Iterate over every combination of the color for the distribution set. loops n^3 times
    for r_delta in distribution {
        for g_delta in distribution {
            for b_delta in distribution {
                // get the current color
                let guass_color = colorspace.to_float(Color::new(
                    (color[0] as f64 + *r_delta) as u8,
                    (color[1] as f64 + *g_delta) as u8,
                    (color[2] as f64 + *b_delta) as u8,
                    255,
                ));

                // remap the temp color
                let idx = color_map.find_nearest(guass_color);
                let nearest = palette[idx];

                // Incremental average
                mean_color.r += nearest.r as f64 / total;
                mean_color.g += nearest.g as f64 / total;
                mean_color.b += nearest.b as f64 / total;
            }
        }
    }

    // Round off and return the final color
    [
        mean_color.r.round() as u8,
        mean_color.g.round() as u8,
        mean_color.b.round() as u8,
    ]
}
