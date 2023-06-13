use exoquant::{Color, ColorMap, ColorSpace};
use image::Rgb;

use super::InterpolatedRemapper;
use crate::{GenerateLut, Image};

/// Simple remapper that doesn't do any interpolation. Mostly used internally by the other
/// algorithms.
pub struct NearestNeighborRemapper<'a, CS: ColorSpace + Sync> {
    palette: &'a [[u8; 3]],
    color_map: ColorMap,
    pub colorspace: CS,
}

impl<'a, CS: ColorSpace + Sync> NearestNeighborRemapper<'a, CS> {
    pub fn new(palette: &'a [[u8; 3]], colorspace: CS) -> Self {
        let color_map = ColorMap::from_float_colors(
            palette
                .iter()
                .map(|c| {
                    colorspace.to_float(Color {
                        r: c[0],
                        g: c[1],
                        b: c[2],
                        a: 255,
                    })
                })
                .collect(),
        );

        Self {
            palette,
            color_map,
            colorspace,
        }
    }
}

impl<'a, CS: ColorSpace + Sync> GenerateLut<'a> for NearestNeighborRemapper<'a, CS> {}
impl<'a, CS: ColorSpace + Sync> InterpolatedRemapper<'a> for NearestNeighborRemapper<'a, CS> {
    fn remap_image(&self, image: &mut Image) {
        for pixel in image.pixels_mut() {
            self.remap_pixel(pixel)
        }
    }

    fn remap_pixel(&self, pixel: &mut Rgb<u8>) {
        let colorf = self
            .colorspace
            .to_float(Color::new(pixel.0[0], pixel.0[1], pixel.0[2], 255));
        let idx = self.color_map.find_nearest(colorf);
        *pixel = Rgb(self.palette[idx]);
    }
}
