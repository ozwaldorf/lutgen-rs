//! The OG way of doing interpolated palette remapping. Based off of the imagemagick solution:
//!
//! ```text
//! convert HALD:8 \
//!     -duplicate 512 \
//!     -attenuate 1 +noise Gaussian \
//!     -quantize LAB +dither -remap <palette.png> \
//!     -evaluate-sequence Mean <output.png>
//! ```

use exoquant::{Color, Remapper, SimpleColorSpace};
use image::{ImageBuffer, Rgb};
use imageproc::noise::gaussian_noise;

use crate::Image;

/// Remap an image into a guassian noise interpolated palette of colors.
///
/// Runs multiple iterations of random gaussian noise, remapping each variant, and finally computing
/// the mean of the sequence of images. The resulting image is an interpolated version of the palette, only
/// using combinations of the original palette colors.
pub fn remap_image(
    image: &Image,
    palette: &[Color],
    mean: f64,
    std_dev: f64,
    iterations: usize,
) -> Image {
    rayon::scope(|s| {
        let (tx, rx) = std::sync::mpsc::channel();
        // Spawn a thread for each iteration
        for i in 0..iterations as u64 {
            let tx = tx.clone();
            s.spawn(move |_| {
                // Apply noise to the image
                let output = gaussian_noise(image, mean, std_dev, i);
                // Remap the variant
                let output = simple_remap(output, palette);
                tx.send(output).unwrap();
            });
        }
        drop(tx);

        // Collect and evaluate the mean image
        sequence_mean(rx.into_iter().collect())
    })
}

/// Simple, non-dithering, nearest neighbor remap.
pub fn simple_remap(mut image: Image, palette: &[Color]) -> Image {
    let width = image.width() as usize;

    let colorspace = SimpleColorSpace::default();
    let ditherer = exoquant::ditherer::None;
    let remapper = Remapper::new(palette, &colorspace, &ditherer);
    let pixels: Vec<Color> = image
        .pixels()
        .map(|c| Color::new(c.0[0], c.0[1], c.0[2], 255))
        .collect();

    // Find nearest neighbor indexes for each pixel
    let palette_indexes = remapper.remap(&pixels, image.width() as usize);

    // Write new pixel values to the image
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let p = y as usize * width + x as usize;
        let color = palette[palette_indexes[p] as usize];
        *pixel = Rgb([color.r, color.g, color.b]);
    }

    image
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
