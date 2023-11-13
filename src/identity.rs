//! Hald clut identity creation and application

use image::Rgb;

use crate::Image;

/// Hald clut base identity generator.
/// Algorithm derived from: <https://www.quelsolaar.com/technology/clut.html>
pub fn generate(level: u8) -> Image {
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

    Image::from_vec(image_size, image_size, buffer).expect("failed to create identity from buffer")
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
