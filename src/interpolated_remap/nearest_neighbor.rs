use exoquant::{Color, ColorMap, ColorSpace};
use image::Rgb;

use super::InterpolatedRemapper;
use crate::Image;

/// Simple remapper that doesn't do any interpolation. Mostly used internally by the other
/// algorithms.
pub struct NearestNeighborRemapper<'a, CS: ColorSpace + Sync> {
    palette: &'a [Color],
    color_map: ColorMap,
    pub colorspace: CS,
}

impl<'a, CS: ColorSpace + Sync> InterpolatedRemapper<'a> for NearestNeighborRemapper<'a, CS> {
    type Params = CS;

    fn new(palette: &'a [Color], colorspace: Self::Params) -> Self {
        let color_map =
            ColorMap::from_float_colors(palette.iter().map(|c| colorspace.to_float(*c)).collect());

        Self {
            palette,
            color_map,
            colorspace,
        }
    }

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
        let nearest = self.palette[idx];

        *pixel = Rgb([nearest.r, nearest.g, nearest.b])
    }
}
