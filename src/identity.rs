//! Hald clut identity creation and application

use image::{ImageBuffer, Pixel};

use crate::{Image, LutImage};

/// Hald clut base identity generator.
/// Algorithm derived from: <https://www.quelsolaar.com/technology/clut.html>
pub fn generate(level: u8) -> LutImage {
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
pub fn correct_pixel<P: Pixel<Subpixel = u8>>(input: &mut P, hald_clut: &LutImage, level: u8) {
    let level = level as u32;
    let cube_size = level * level;

    let [r, g, b, ..] = input.channels_mut() else {
        panic!("pixel must have 3 channels")
    };

    let rs = *r as u32 * (cube_size - 1) / 255;
    let gs = *g as u32 * (cube_size - 1) / 255;
    let bs = *b as u32 * (cube_size - 1) / 255;

    let x = (rs % cube_size) + (gs % level) * cube_size;
    let y = (bs * level) + (gs / level);

    [*r, *g, *b] = hald_clut.get_pixel(x, y).0;
}

/// Correct an image with a hald clut identity in place.
/// Panics if the hald clut is invalid.
///
/// Simple implementation that doesn't do any interpolation,
/// so higher LUT sizes will prove to be more accurate.
///
/// # Panics
///
/// If the hald clut is not a square or valid size for the level
pub fn correct_image<P: Pixel<Subpixel = u8>>(image: &mut Image<P>, hald_clut: &LutImage) {
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
        correct_pixel(pixel, hald_clut, level as u8);
    }
}
