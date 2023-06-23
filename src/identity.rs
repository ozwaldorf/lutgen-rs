//! Hald clut identity creation and application

use image::{ImageBuffer, Rgb};

use crate::Image;

/// Hald clut base identity generator.
/// Algorithm derived from: <https://www.quelsolaar.com/technology/clut.html>
pub fn generate(level: u8) -> Image {
    let level = level as u32;
    let cube_size = level * level;
    let image_size = cube_size * level;

    let mut imgbuf = ImageBuffer::new(image_size, image_size);

    let mut p = 0u32;
    for blue in 0..cube_size {
        for green in 0..cube_size {
            for red in 0..cube_size {
                let r = red * 255 / (cube_size - 1);
                let g = green * 255 / (cube_size - 1);
                let b = blue * 255 / (cube_size - 1);
                let pixel = image::Rgb([r as u8, g as u8, b as u8]);

                let x = p % image_size;
                let y = (p - x) / image_size;

                imgbuf.put_pixel(x, y, pixel);
                p += 1;
            }
        }
    }

    imgbuf
}

/// Correct a single pixel with a hald clut identity.
///
/// Simple implementation that doesn't do any interpolation,
/// so higher LUT sizes will prove to be more accurate.
pub fn correct_pixel(input: &[u8; 3], hald_clut: &Image, level: u8) -> [u8; 3] {
    let level = level as u32;
    let cube_size = level * level;

    let r = input[0] as u32 * (cube_size - 1) / 255;
    let g = input[1] as u32 * (cube_size - 1) / 255;
    let b = input[2] as u32 * (cube_size - 1) / 255;

    let x = (r % cube_size) + (g % level) * cube_size;
    let y = (b * level) + (g / level);

    hald_clut.get_pixel(x, y).0
}

/// Correct an image with a hald clut identity in place.
/// Panics if the hald clut is invalid.
///
/// Simple implementation that doesn't do any interpolation,
/// so higher LUT sizes will prove to be more accurate.
pub fn correct_image(image: &mut Image, hald_clut: &Image) {
    let (width, height) = hald_clut.dimensions();

    // Find the smallest level that fits inside the hald clut
    let mut level = 2;
    while level * level * level < width {
        level += 1;
    }

    // Ensure the hald clut is valid for the calculated level
    assert_eq!(width, level * level * level);
    assert_eq!(width, height);

    // Correct the original pixels
    for pixel in image.pixels_mut() {
        *pixel = Rgb(correct_pixel(&pixel.0, hald_clut, level as u8));
    }
}
