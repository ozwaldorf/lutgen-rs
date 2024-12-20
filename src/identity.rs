//! Hald clut identity creation and application

use crate::{RgbImage, RgbaImage};

/// Hald clut base identity generator.
/// Algorithm derived from: <https://www.quelsolaar.com/technology/clut.html>
pub fn generate(level: u8) -> RgbImage {
    let level = level as u32;
    let cube_size = level * level;
    let image_size = cube_size * level;

    let mut buffer = vec![0; (image_size * image_size * 3) as usize];

    let mut i = 0;
    for blue in 0..cube_size {
        let b = (blue * 255 / (cube_size - 1)) as u8;
        for green in 0..cube_size {
            let g = (green * 255 / (cube_size - 1)) as u8;
            for red in 0..cube_size {
                let r = (red * 255 / (cube_size - 1)) as u8;

                buffer[i] = r;
                i += 1;
                buffer[i] = g;
                i += 1;
                buffer[i] = b;
                i += 1;
            }
        }
    }

    RgbImage::from_vec(image_size, image_size, buffer)
        .expect("failed to create identity from buffer")
}

/// Correct a single pixel with a hald clut identity.
///
/// Simple implementation that doesn't do any interpolation,
/// so higher LUT sizes will prove to be more accurate.
pub fn correct_pixel(input: &[u8; 3], hald_clut: &RgbImage, level: u8) -> [u8; 3] {
    let level = level as u32;
    let cube_size = level * level;

    let r = input[0] as u32 * (cube_size - 1) / 255;
    let g = input[1] as u32 * (cube_size - 1) / 255;
    let b = input[2] as u32 * (cube_size - 1) / 255;

    let x = (r % cube_size) + (g % level) * cube_size;
    let y = (b * level) + (g / level);

    hald_clut.get_pixel(x, y).0
}

/// Correct an image in place with a hald clut identity.
///
/// Simple implementation that doesn't do any interpolation,
/// so higher LUT sizes will prove to be more accurate.
///
/// # Safety
///
/// Panics if the hald clut is invalid.
pub fn correct_image(image: &mut RgbaImage, hald_clut: &RgbImage) {
    let level = detect_level(hald_clut);
    for pixel in image.pixels_mut() {
        let [r, g, b] = correct_pixel(&[pixel[0], pixel[1], pixel[2]], hald_clut, level);
        pixel.0[0] = r;
        pixel.0[1] = g;
        pixel.0[2] = b;
    }
}

/// Detect a hald clut identities level.
///
/// # Safety
///
/// Panics if the hald clut is invalid.
pub fn detect_level(hald_clut: &RgbImage) -> u8 {
    let (width, height) = hald_clut.dimensions();

    // Find the smallest level that fits inside the hald clut
    let mut level = 2;
    while level * level * level < width {
        level += 1;
    }

    // Ensure the hald clut is valid for the calculated level
    assert_eq!(width, level * level * level);
    assert_eq!(width, height);

    level as u8
}
