//! Hald clut identity creation and application

use crate::{RgbImage, RgbaImage};

/// Pixels per chunk for parallel processing.
/// 256 pixels = 1KB output, balances scheduling overhead vs load balancing.
const CHUNK_SIZE: usize = 256;

/// Optimized sampler for Hald CLUT lookup with interpolation support.
///
/// Converts the 2D Hald CLUT to a 3D array for direct indexing (no modulo/division per sample).
/// Supports nearest-neighbor (fast), trilinear (smooth), and tetrahedral (balanced) interpolation.
#[derive(Clone)]
pub struct HaldClutSampler {
    /// Flattened 3D LUT: index = r + g * cube_size + b * cube_size²
    lut: Vec<[u8; 3]>,
    cube_size: u32,
}

impl HaldClutSampler {
    /// Create a new sampler from a Hald CLUT image.
    ///
    /// Converts the 2D Hald layout to a 3D array for faster lookups.
    ///
    /// # Panics
    ///
    /// Panics if the hald clut dimensions are invalid.
    pub fn new(hald_clut: &RgbImage) -> Self {
        Self::new_with_level(hald_clut, detect_level(hald_clut))
    }

    /// Create a new sampler with a known level.
    ///
    /// Converts the 2D Hald layout to a 3D array for faster lookups.
    pub fn new_with_level(hald_clut: &RgbImage, level: u8) -> Self {
        let level = level as u32;
        let cube_size = level * level;
        let total = (cube_size * cube_size * cube_size) as usize;

        let mut lut = vec![[0u8; 3]; total];

        // Convert from 2D Hald layout to linear 3D array
        for b in 0..cube_size {
            for g in 0..cube_size {
                for r in 0..cube_size {
                    let x = (r % cube_size) + (g % level) * cube_size;
                    let y = (b * level) + (g / level);
                    let src = hald_clut.get_pixel(x, y).0;
                    let idx = (r + g * cube_size + b * cube_size * cube_size) as usize;
                    lut[idx] = src;
                }
            }
        }

        Self { lut, cube_size }
    }

    /// Get the cube size (level²).
    #[inline]
    pub fn cube_size(&self) -> u32 {
        self.cube_size
    }

    /// Internal: sample at specific cube coordinates via direct indexing.
    #[inline(always)]
    fn sample_at(&self, r: u32, g: u32, b: u32) -> [u8; 3] {
        let idx = (r + g * self.cube_size + b * self.cube_size * self.cube_size) as usize;
        self.lut[idx]
    }

    /// Sample using nearest-neighbor lookups (fastest, quality depends on LUT size).
    #[inline]
    pub fn sample_nearest(&self, rgb: [u8; 3]) -> [u8; 3] {
        let r = rgb[0] as u32 * (self.cube_size - 1) / 255;
        let g = rgb[1] as u32 * (self.cube_size - 1) / 255;
        let b = rgb[2] as u32 * (self.cube_size - 1) / 255;
        self.sample_at(r, g, b)
    }

    /// Sample with trilinear interpolation (smoother, higher quality).
    #[inline]
    pub fn sample_trilinear(&self, rgb: [u8; 3]) -> [u8; 3] {
        let scale = (self.cube_size - 1) as f32 / 255.0;

        // Compute floating-point coordinates in cube space
        let rf = rgb[0] as f32 * scale;
        let gf = rgb[1] as f32 * scale;
        let bf = rgb[2] as f32 * scale;

        // Get integer coordinates and fractions
        let r0 = rf as u32;
        let g0 = gf as u32;
        let b0 = bf as u32;

        let r1 = (r0 + 1).min(self.cube_size - 1);
        let g1 = (g0 + 1).min(self.cube_size - 1);
        let b1 = (b0 + 1).min(self.cube_size - 1);

        let fr = rf.fract();
        let fg = gf.fract();
        let fb = bf.fract();

        // Sample 8 corners of the cube
        let c000 = self.sample_at(r0, g0, b0);
        let c100 = self.sample_at(r1, g0, b0);
        let c010 = self.sample_at(r0, g1, b0);
        let c110 = self.sample_at(r1, g1, b0);
        let c001 = self.sample_at(r0, g0, b1);
        let c101 = self.sample_at(r1, g0, b1);
        let c011 = self.sample_at(r0, g1, b1);
        let c111 = self.sample_at(r1, g1, b1);

        // Trilinear interpolation
        let inv_fr = 1.0 - fr;
        let inv_fg = 1.0 - fg;
        let inv_fb = 1.0 - fb;

        let mut result = [0u8; 3];
        for i in 0..3 {
            let c00 = c000[i] as f32 * inv_fr + c100[i] as f32 * fr;
            let c01 = c001[i] as f32 * inv_fr + c101[i] as f32 * fr;
            let c10 = c010[i] as f32 * inv_fr + c110[i] as f32 * fr;
            let c11 = c011[i] as f32 * inv_fr + c111[i] as f32 * fr;

            let c0 = c00 * inv_fg + c10 * fg;
            let c1 = c01 * inv_fg + c11 * fg;

            let c = c0 * inv_fb + c1 * fb;
            result[i] = c.round() as u8;
        }

        result
    }

    /// Sample with tetrahedral interpolation (faster than trilinear, good quality).
    #[inline]
    pub fn sample_tetrahedral(&self, rgb: [u8; 3]) -> [u8; 3] {
        let scale = (self.cube_size - 1) as f32 / 255.0;

        let rf = rgb[0] as f32 * scale;
        let gf = rgb[1] as f32 * scale;
        let bf = rgb[2] as f32 * scale;

        let r0 = rf as u32;
        let g0 = gf as u32;
        let b0 = bf as u32;

        let r1 = (r0 + 1).min(self.cube_size - 1);
        let g1 = (g0 + 1).min(self.cube_size - 1);
        let b1 = (b0 + 1).min(self.cube_size - 1);

        let fr = rf.fract();
        let fg = gf.fract();
        let fb = bf.fract();

        // Base vertex
        let c000 = self.sample_at(r0, g0, b0);

        // Determine which tetrahedron and get vertices
        let (v1, v2, v3, w1, w2, w3) = if fr > fg {
            if fg > fb {
                // R > G > B
                let c100 = self.sample_at(r1, g0, b0);
                let c110 = self.sample_at(r1, g1, b0);
                let c111 = self.sample_at(r1, g1, b1);
                (c100, c110, c111, fr, fg, fb)
            } else if fr > fb {
                // R > B > G
                let c100 = self.sample_at(r1, g0, b0);
                let c101 = self.sample_at(r1, g0, b1);
                let c111 = self.sample_at(r1, g1, b1);
                (c100, c101, c111, fr, fb, fg)
            } else {
                // B > R > G
                let c001 = self.sample_at(r0, g0, b1);
                let c101 = self.sample_at(r1, g0, b1);
                let c111 = self.sample_at(r1, g1, b1);
                (c001, c101, c111, fb, fr, fg)
            }
        } else if fb > fg {
            // B > G > R
            let c001 = self.sample_at(r0, g0, b1);
            let c011 = self.sample_at(r0, g1, b1);
            let c111 = self.sample_at(r1, g1, b1);
            (c001, c011, c111, fb, fg, fr)
        } else if fb > fr {
            // G > B > R
            let c010 = self.sample_at(r0, g1, b0);
            let c011 = self.sample_at(r0, g1, b1);
            let c111 = self.sample_at(r1, g1, b1);
            (c010, c011, c111, fg, fb, fr)
        } else {
            // G > R > B
            let c010 = self.sample_at(r0, g1, b0);
            let c110 = self.sample_at(r1, g1, b0);
            let c111 = self.sample_at(r1, g1, b1);
            (c010, c110, c111, fg, fr, fb)
        };

        // Barycentric interpolation
        let w0 = 1.0 - w1;
        let dw1 = w1 - w2;
        let dw2 = w2 - w3;

        let mut result = [0u8; 3];
        for i in 0..3 {
            let c =
                c000[i] as f32 * w0 + v1[i] as f32 * dw1 + v2[i] as f32 * dw2 + v3[i] as f32 * w3;
            result[i] = c.clamp(0.0, 255.0).round() as u8;
        }

        result
    }

    /// Correct an image using nearest-neighbor lookup.
    #[inline]
    pub fn correct_image(&self, image: &mut RgbaImage) {
        for pixel in image.pixels_mut() {
            let [r, g, b] = self.sample_nearest([pixel[0], pixel[1], pixel[2]]);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }
    }

    /// Correct an image using trilinear interpolation.
    #[inline]
    pub fn correct_image_trilinear(&self, image: &mut RgbaImage) {
        for pixel in image.pixels_mut() {
            let [r, g, b] = self.sample_trilinear([pixel[0], pixel[1], pixel[2]]);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }
    }

    /// Correct an image using tetrahedral interpolation.
    #[inline]
    pub fn correct_image_tetrahedral(&self, image: &mut RgbaImage) {
        for pixel in image.pixels_mut() {
            let [r, g, b] = self.sample_tetrahedral([pixel[0], pixel[1], pixel[2]]);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }
    }

    /// Parallel: correct an image using nearest-neighbor lookup.
    #[cfg(feature = "rayon")]
    #[inline]
    pub fn par_correct_image(&self, image: &mut RgbaImage) {
        use rayon::prelude::*;
        image
            .as_mut()
            .par_chunks_mut(CHUNK_SIZE * 4)
            .for_each(|chunk| {
                for pixel in chunk.chunks_exact_mut(4) {
                    let [r, g, b] = self.sample_nearest([pixel[0], pixel[1], pixel[2]]);
                    pixel[0] = r;
                    pixel[1] = g;
                    pixel[2] = b;
                }
            });
    }

    /// Parallel: correct an image using trilinear interpolation.
    #[cfg(feature = "rayon")]
    #[inline]
    pub fn par_correct_image_trilinear(&self, image: &mut RgbaImage) {
        use rayon::prelude::*;
        image
            .as_mut()
            .par_chunks_mut(CHUNK_SIZE * 4)
            .for_each(|chunk| {
                for pixel in chunk.chunks_exact_mut(4) {
                    let [r, g, b] = self.sample_trilinear([pixel[0], pixel[1], pixel[2]]);
                    pixel[0] = r;
                    pixel[1] = g;
                    pixel[2] = b;
                }
            });
    }

    /// Parallel: correct an image using tetrahedral interpolation.
    #[cfg(feature = "rayon")]
    #[inline]
    pub fn par_correct_image_tetrahedral(&self, image: &mut RgbaImage) {
        use rayon::prelude::*;
        image
            .as_mut()
            .par_chunks_mut(CHUNK_SIZE * 4)
            .for_each(|chunk| {
                for pixel in chunk.chunks_exact_mut(4) {
                    let [r, g, b] = self.sample_tetrahedral([pixel[0], pixel[1], pixel[2]]);
                    pixel[0] = r;
                    pixel[1] = g;
                    pixel[2] = b;
                }
            });
    }
}

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

/// Correct a single pixel with a hald clut.
///
/// Uses nearest-neighbor sampling.
#[deprecated(
    since = "0.15.0",
    note = "Use HaldClutSampler::sample_nearest/trilinear/tetrahedral instead"
)]
#[inline]
pub fn correct_pixel(pixel: &[u8; 3], hald_clut: &RgbImage, level: u8) -> [u8; 3] {
    HaldClutSampler::new_with_level(hald_clut, level).sample_nearest(*pixel)
}

/// Correct an image in place with a hald clut identity.
///
/// # Panics
///
/// Panics if the hald clut is invalid.
#[deprecated(since = "0.15.0", note = "Use HaldClutSampler::correct_image instead")]
#[inline]
pub fn correct_image(image: &mut RgbaImage, hald_clut: &RgbImage) {
    HaldClutSampler::new(hald_clut).correct_image(image);
}

/// Detect a hald clut identities level.
///
/// # Panics
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
