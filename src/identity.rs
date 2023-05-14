//! Hald CLUT base identity generation

use image::ImageBuffer;

use crate::Image;

/// Hald CLUT identity generator.
/// Algorithm derived from: <https://www.quelsolaar.com/technology/clut.html>
pub fn generate(level: u32) -> Image {
    let cube_size = level * level;
    let image_size = cube_size * level;

    let mut imgbuf = ImageBuffer::new(image_size, image_size);

    let mut p = 0.0;
    for blue in 0..cube_size {
        for green in 0..cube_size {
            for red in 0..cube_size {
                let r = ((red as f64) / (cube_size - 1) as f64) * 255.0;
                let g = ((green as f64) / (cube_size - 1) as f64) * 255.0;
                let b = ((blue as f64) / (cube_size - 1) as f64) * 255.0;
                let pixel = image::Rgb([r as u8, g as u8, b as u8]);

                let x = p % image_size as f64;
                let y = (p - x) / image_size as f64;
                imgbuf.put_pixel(x as u32, y as u32, pixel);

                p += 1.0;
            }
        }
    }

    imgbuf
}
