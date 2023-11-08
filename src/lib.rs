#![doc = include_str!("../README.md")]
#![allow(long_running_const_eval)]

use image::{ImageBuffer, Rgb};
use interpolation::InterpolatedRemapper;
use rayon::{
    prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::{ParallelSlice, ParallelSliceMut},
};

pub mod identity;
pub mod interpolation;

/// Core image type (Rgb8)
pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

pub trait GenerateLut<'a>: InterpolatedRemapper<'a> {
    /// Helper method to generate a lut using an [`InterpolatedRemapper`].
    fn generate_lut(&self, level: u8) -> Image {
        let level = level as u32;
        let cube_size = level * level;
        let cube_sqr = cube_size * cube_size;
        let image_size = cube_size * level;

        let cube_size_minus_one = cube_size - 1;

        let mut buffer = vec![0u8; (image_size * image_size * 3) as usize];

        buffer
            .par_chunks_exact_mut(3)
            .enumerate()
            .for_each(|(i, pixel)| {
                let i = i as u32;

                // OPTION THAT ONLY WORKS WITH POWERS OF 2
                // let r = i & cube_size_minus_one;
                // let r = ((r << 8) - r) / cube_size_minus_one;
                //
                // let g = (i >> cube_size_log2) & cube_size_minus_one;
                // let g = ((g << 8) - g) / cube_size_minus_one;
                //
                // let b = (i >> cube_sqr_log2) & cube_size_minus_one;
                // let b = ((b << 8) - b) / cube_size_minus_one;

                let r = (i % cube_size) * 255 / cube_size_minus_one;
                let g = ((i / cube_size) % cube_size) * 255 / cube_size_minus_one;
                let b = ((i / cube_sqr) % cube_size) * 255 / cube_size_minus_one;

                let mut pix = image::Rgb([r as u8, g as u8, b as u8]);
                self.remap_pixel(&mut pix);
                pixel[0] = pix[0];
                pixel[1] = pix[1];
                pixel[2] = pix[2];
            });

        Image::from_vec(image_size, image_size, buffer).unwrap()
    }
}
