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
        let image_size = cube_size * level;

        let cube_size_minus_one = cube_size - 1;
        let cube_size_log2 = cube_size.ilog2();

        let mut buffer = vec![0u8; (image_size * image_size * 3) as usize];

        let mut normalization_lut = [0u32; 512];
        for i in 0..cube_size {
            normalization_lut[i as usize] = 255 * i / cube_size_minus_one;
        }

        buffer
            .par_chunks_exact_mut(3)
            .enumerate()
            .for_each(|(i, pixel)| {
                let mut i = i as u32;

                let r =
                    unsafe { *normalization_lut.get_unchecked((i & cube_size_minus_one) as usize) };
                i >>= cube_size_log2;
                let g =
                    unsafe { *normalization_lut.get_unchecked((i & cube_size_minus_one) as usize) };
                i >>= cube_size_log2;
                let b =
                    unsafe { *normalization_lut.get_unchecked((i & cube_size_minus_one) as usize) };

                let mut pix = image::Rgb([r as u8, g as u8, b as u8]);
                self.remap_pixel(&mut pix);
                pixel[0] = pix[0];
                pixel[1] = pix[1];
                pixel[2] = pix[2];
            });

        Image::from_vec(image_size, image_size, buffer).unwrap()
    }
}
