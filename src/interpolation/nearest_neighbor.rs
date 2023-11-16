use image::Rgb;
use kiddo::{NearestNeighbour, SquaredEuclidean};
use oklab::{srgb_to_oklab, Oklab};

use super::{ColorTree, InterpolatedRemapper};
use crate::{GenerateLut, Image};

/// Simple remapper that doesn't do any interpolation. Mostly used internally by the other
/// algorithms.
pub struct NearestNeighborRemapper<'a> {
    palette: &'a [[u8; 3]],
    tree: ColorTree,
    lum_factor: f64,
}

impl<'a> NearestNeighborRemapper<'a> {
    pub fn new(palette: &'a [[u8; 3]], lum_factor: f64) -> Self {
        let mut tree = ColorTree::new();
        for (i, &color) in palette.iter().enumerate() {
            let Oklab { l, a, b } = srgb_to_oklab(color.into());
            tree.add(&[l as f64 * lum_factor, a as f64, b as f64], i as u32);
        }

        Self {
            palette,
            tree,
            lum_factor,
        }
    }
}

impl<'a> GenerateLut<'a> for NearestNeighborRemapper<'a> {}
impl<'a> InterpolatedRemapper<'a> for NearestNeighborRemapper<'a> {
    fn remap_image(&self, image: &mut Image) {
        for pixel in image.pixels_mut() {
            self.remap_pixel(pixel)
        }
    }

    fn remap_pixel(&self, pixel: &mut Rgb<u8>) {
        let Oklab { l, a, b } = srgb_to_oklab(pixel.0.into());
        let NearestNeighbour { item, .. } = self.tree.nearest_one::<SquaredEuclidean>(&[
            l as f64 * self.lum_factor,
            a as f64,
            b as f64,
        ]);
        *pixel = Rgb(self.palette[item as usize]);
    }
}
