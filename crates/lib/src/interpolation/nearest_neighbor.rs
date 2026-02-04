use arrayref::array_ref;
use image::Rgba;
use kiddo::{NearestNeighbour, SquaredEuclidean};
use oklab::{oklab_to_srgb, srgb_to_oklab, Oklab};

use super::{ColorTree, InterpolatedRemapper};
use crate::RgbaImage;

/// Simple remapper that doesn't do any interpolation. Mostly used internally by the other
/// algorithms.
pub struct NearestNeighborRemapper<'a> {
    palette: &'a [[u8; 3]],
    preserve: Vec<(f32, f32)>,
    tree: ColorTree,
    lum_factor: f64,
}

impl<'a> NearestNeighborRemapper<'a> {
    pub fn new(palette: &'a [[u8; 3]], lum_factor: f64, preserve: bool) -> Self {
        let mut tree = ColorTree::new();
        let mut preserve_buf = Vec::new();
        for (i, &color) in palette.iter().enumerate() {
            let Oklab { l, a, b } = srgb_to_oklab(color.into());
            tree.add(&[l as f64 * lum_factor, a as f64, b as f64], i as u32);
            if preserve {
                preserve_buf.push((a, b));
            }
        }

        Self {
            palette,
            tree,
            lum_factor,
            preserve: preserve_buf,
        }
    }
}

impl<'a> InterpolatedRemapper<'a> for NearestNeighborRemapper<'a> {
    fn remap_image(&self, image: &mut RgbaImage) {
        for pixel in image.pixels_mut() {
            self.remap_pixel(pixel)
        }
    }

    fn remap_pixel(&self, pixel: &mut Rgba<u8>) {
        let Oklab { l, a, b } = srgb_to_oklab((*array_ref![pixel.0, 0, 3]).into());
        let NearestNeighbour { item, .. } = self.tree.nearest_one::<SquaredEuclidean>(&[
            l as f64 * self.lum_factor,
            a as f64,
            b as f64,
        ]);

        if self.preserve.is_empty() {
            pixel.0[0..3].copy_from_slice(&self.palette[item as usize]);
        } else {
            let (a, b) = self.preserve[item as usize];
            let rgb = oklab_to_srgb(Oklab { l, a, b });
            pixel.0[0..3].copy_from_slice(rgb.as_ref());
        }
    }
}
