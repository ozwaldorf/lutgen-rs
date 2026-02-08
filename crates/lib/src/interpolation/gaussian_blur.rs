//! Gaussian blur LUT remapping in OKLab color space.
//!
//! Creates a nearest-neighbor LUT with OKLab colors,
//! then applies separable Gaussian blur directly on the color values.
//!
//! Uses a transpose-based approach for optimal cache locality:
//! each blur pass operates on contiguous memory (stride=1), then
//! the dimensions are rotated so the next axis becomes contiguous.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use image::RgbImage;
use oklab::{oklab_to_srgb, srgb_to_oklab, Oklab};
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::GenerateLut;

/// Remapper using separable Gaussian blur on OKLab colors.
///
/// Algorithm:
/// 1. Build nearest-neighbor LUT storing OKLab colors
/// 2. Apply separable Gaussian blur directly on L, a, b channels
/// 3. Convert back to RGB
///
/// Uses transpose-based cache optimization: each blur pass operates on
/// contiguous memory, with dimension rotations between passes.
pub struct GaussianBlurRemapper {
    palette_oklab: Vec<[f32; 3]>,
    radius: f32,
    lum_factor: f32,
    preserve: bool,
}

impl GaussianBlurRemapper {
    #[inline]
    pub fn new(palette: &[[u8; 3]], radius: f64, lum_factor: f64, preserve: bool) -> Self {
        let lum_factor = lum_factor as f32;

        let palette_oklab: Vec<[f32; 3]> = palette
            .iter()
            .map(|&color| {
                let Oklab { l, a, b } = srgb_to_oklab(color.into());
                [l * lum_factor, a, b]
            })
            .collect();

        Self {
            palette_oklab,
            radius: radius as f32,
            lum_factor,
            preserve,
        }
    }

    /// Find nearest palette color, using a hint from spatial coherence.
    /// Checks hint first; if distance is below threshold, skips full scan.
    #[inline(always)]
    fn find_nearest_with_hint(&self, color: [f32; 3], hint: usize, threshold_sq: f32) -> usize {
        let hint_dist = sq_dist(color, self.palette_oklab[hint]);

        // If hint is very close, it's almost certainly still the best
        if hint_dist < threshold_sq {
            return hint;
        }

        // Full scan starting from hint as best candidate
        let mut best_idx = hint;
        let mut best_dist = hint_dist;

        for (i, target) in self.palette_oklab.iter().enumerate() {
            if i == hint {
                continue;
            }
            let d = sq_dist(color, *target);
            if d < best_dist {
                best_dist = d;
                best_idx = i;
            }
        }
        best_idx
    }

    #[inline(always)]
    fn cell_idx(r: usize, g: usize, b: usize, size: usize) -> usize {
        (r * size + g) * size + b
    }

    fn build_kernel(&self) -> Vec<f32> {
        let half = (self.radius * 3.0).ceil() as i32;
        let two_sigma_sq = 2.0 * self.radius * self.radius;
        let mut kernel: Vec<f32> = (-half..=half)
            .map(|i| (-(i * i) as f32 / two_sigma_sq).exp())
            .collect();
        let sum: f32 = kernel.iter().sum();
        kernel.iter_mut().for_each(|w| *w /= sum);
        kernel
    }

    /// Apply 1D Gaussian blur along the innermost (contiguous) axis.
    /// This has optimal cache locality with stride=1 access pattern.
    fn blur_inner(
        src: &[f32],
        dst: &mut [f32],
        size: usize,
        channels: usize,
        kernel: &[f32],
        half: i32,
        max: i32,
    ) {
        let row_len = size * channels;

        for outer in 0..size {
            for mid in 0..size {
                let row_base = (outer * size + mid) * row_len;
                for inner in 0..size {
                    let out_base = row_base + inner * channels;
                    for c in 0..channels {
                        let mut sum = 0.0f32;
                        for (ki, &kw) in kernel.iter().enumerate() {
                            let inner_src =
                                (inner as i32 + ki as i32 - half).clamp(0, max) as usize;
                            let idx_in = row_base + inner_src * channels + c;
                            sum += kw * src[idx_in];
                        }
                        dst[out_base + c] = sum;
                    }
                }
            }
        }
    }

    #[cfg(feature = "rayon")]
    fn par_blur_inner(
        src: &[f32],
        dst: &mut [f32],
        size: usize,
        channels: usize,
        kernel: &[f32],
        half: i32,
        max: i32,
    ) {
        let row_len = size * channels;

        // Parallelize over rows (outer * size + mid)
        dst.par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(row_idx, row_out)| {
                let row_base = row_idx * row_len;
                for inner in 0..size {
                    let out_base = inner * channels;
                    for c in 0..channels {
                        let mut sum = 0.0f32;
                        for (ki, &kw) in kernel.iter().enumerate() {
                            let inner_src =
                                (inner as i32 + ki as i32 - half).clamp(0, max) as usize;
                            let idx_in = row_base + inner_src * channels + c;
                            sum += kw * src[idx_in];
                        }
                        row_out[out_base + c] = sum;
                    }
                }
            });
    }

    /// Rotate dimensions: [a][b][c] -> [b][c][a]
    /// After rotation, the previously outermost dimension becomes innermost.
    fn rotate_dims(src: &[f32], dst: &mut [f32], size: usize, channels: usize) {
        for a in 0..size {
            for b in 0..size {
                for c in 0..size {
                    let src_idx = ((a * size + b) * size + c) * channels;
                    let dst_idx = ((b * size + c) * size + a) * channels;
                    for ch in 0..channels {
                        dst[dst_idx + ch] = src[src_idx + ch];
                    }
                }
            }
        }
    }

    #[cfg(feature = "rayon")]
    fn par_rotate_dims(src: &[f32], dst: &mut [f32], size: usize, channels: usize) {
        let row_len = size * channels;

        // Parallelize over destination rows
        dst.par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(dst_row, row_out)| {
                let b = dst_row / size;
                let c = dst_row % size;
                for a in 0..size {
                    let src_idx = ((a * size + b) * size + c) * channels;
                    let dst_local = a * channels;
                    for ch in 0..channels {
                        row_out[dst_local + ch] = src[src_idx + ch];
                    }
                }
            });
    }

    fn generate_lut_inner(&self, level: u8, abort: Option<&AtomicBool>) -> Option<RgbImage> {
        let size = (level as usize).pow(2);
        let n_cells = size * size * size;
        let scale = 255.0 / (size - 1) as f32;

        // Build NN LUT with OKLab colors (only a, b if preserve mode)
        let channels = if self.preserve { 2 } else { 3 };
        let mut colors: Vec<f32> = Vec::with_capacity(n_cells * channels);

        // Threshold for early-exit: squared distance in OKLab space
        // Adjacent LUT cells differ by ~1/size in each RGB channel
        // In OKLab, this is roughly 0.01-0.02 per step, so threshold ~0.001 sq dist
        let step = 1.0 / size as f32;
        let threshold_sq = step * step * 0.5;

        let mut hint = 0usize;
        for r in 0..size {
            let rf = (r as f32 * scale).round() as u8;
            for g in 0..size {
                let gf = (g as f32 * scale).round() as u8;
                for b in 0..size {
                    let bf = (b as f32 * scale).round() as u8;
                    let Oklab { l, a, b: ob } = srgb_to_oklab([rf, gf, bf].into());
                    let nearest = self.find_nearest_with_hint(
                        [l * self.lum_factor, a, ob],
                        hint,
                        threshold_sq,
                    );
                    hint = nearest;
                    let target = &self.palette_oklab[nearest];

                    if self.preserve {
                        colors.push(target[1]); // a
                        colors.push(target[2]); // b
                    } else {
                        colors.push(target[0] / self.lum_factor);
                        colors.push(target[1]);
                        colors.push(target[2]);
                    }
                }
            }
        }

        let mut colors_next = vec![0.0f32; n_cells * channels];
        let kernel = self.build_kernel();
        let half = (kernel.len() / 2) as i32;
        let max = size as i32 - 1;

        // Pass 1: Blur along B axis (innermost, stride=1)
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::blur_inner(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            half,
            max,
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Rotate [R][G][B] -> [G][B][R], now R is innermost
        Self::rotate_dims(&colors, &mut colors_next, size, channels);
        std::mem::swap(&mut colors, &mut colors_next);

        // Pass 2: Blur along R axis (now innermost, stride=1)
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::blur_inner(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            half,
            max,
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Rotate [G][B][R] -> [B][R][G], now G is innermost
        Self::rotate_dims(&colors, &mut colors_next, size, channels);
        std::mem::swap(&mut colors, &mut colors_next);

        // Pass 3: Blur along G axis (now innermost, stride=1)
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::blur_inner(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            half,
            max,
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Rotate [B][R][G] -> [R][G][B], back to original layout
        Self::rotate_dims(&colors, &mut colors_next, size, channels);

        self.colors_to_lut(&colors_next, size, channels, level)
    }

    #[cfg(feature = "rayon")]
    fn par_generate_lut_inner(&self, level: u8, abort: Option<&AtomicBool>) -> Option<RgbImage> {
        let size = (level as usize).pow(2);
        let n_cells = size * size * size;
        let scale = 255.0 / (size - 1) as f32;

        let channels = if self.preserve { 2 } else { 3 };

        // Build NN LUT with OKLab colors in parallel
        // Parallelize over rows (r, g) to maintain spatial coherence along b axis
        let step = 1.0 / size as f32;
        let threshold_sq = step * step * 0.5;
        let row_len = size * channels;

        let mut colors: Vec<f32> = vec![0.0; n_cells * channels];
        colors
            .par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(row_idx, row)| {
                let g = row_idx % size;
                let r = row_idx / size;
                let rf = (r as f32 * scale).round() as u8;
                let gf = (g as f32 * scale).round() as u8;

                let mut hint = 0usize;
                for b in 0..size {
                    let bf = (b as f32 * scale).round() as u8;
                    let Oklab { l, a, b: ob } = srgb_to_oklab([rf, gf, bf].into());
                    let nearest = self.find_nearest_with_hint(
                        [l * self.lum_factor, a, ob],
                        hint,
                        threshold_sq,
                    );
                    hint = nearest;
                    let target = &self.palette_oklab[nearest];

                    let out_base = b * channels;
                    if self.preserve {
                        row[out_base] = target[1];
                        row[out_base + 1] = target[2];
                    } else {
                        row[out_base] = target[0] / self.lum_factor;
                        row[out_base + 1] = target[1];
                        row[out_base + 2] = target[2];
                    }
                }
            });

        let mut colors_next = vec![0.0f32; n_cells * channels];
        let kernel = self.build_kernel();
        let half = (kernel.len() / 2) as i32;
        let max = size as i32 - 1;

        // Pass 1: Blur along B axis (innermost, stride=1)
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::par_blur_inner(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            half,
            max,
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Rotate [R][G][B] -> [G][B][R], now R is innermost
        Self::par_rotate_dims(&colors, &mut colors_next, size, channels);
        std::mem::swap(&mut colors, &mut colors_next);

        // Pass 2: Blur along R axis (now innermost, stride=1)
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::par_blur_inner(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            half,
            max,
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Rotate [G][B][R] -> [B][R][G], now G is innermost
        Self::par_rotate_dims(&colors, &mut colors_next, size, channels);
        std::mem::swap(&mut colors, &mut colors_next);

        // Pass 3: Blur along G axis (now innermost, stride=1)
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::par_blur_inner(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            half,
            max,
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Rotate [B][R][G] -> [R][G][B], back to original layout
        Self::par_rotate_dims(&colors, &mut colors_next, size, channels);

        self.par_colors_to_lut(&colors_next, size, channels, level)
    }

    /// Convert a pixel index (HALD CLUT order: b outer, g, r inner) to (r, g, b) cell coords.
    #[inline(always)]
    fn pixel_to_rgb(pixel_idx: usize, size: usize) -> (usize, usize, usize) {
        let r_idx = pixel_idx % size;
        let g_idx = (pixel_idx / size) % size;
        let b_idx = pixel_idx / (size * size);
        (r_idx, g_idx, b_idx)
    }

    /// Convert an OKLab color at the given cell to an sRGB pixel.
    #[inline(always)]
    fn cell_to_rgb(
        colors: &[f32],
        idx: usize,
        preserve: bool,
        scale: f32,
        r_idx: usize,
        g_idx: usize,
        b_idx: usize,
    ) -> oklab::Rgb<u8> {
        if preserve {
            let rf = (r_idx as f32 * scale).round() as u8;
            let gf = (g_idx as f32 * scale).round() as u8;
            let bf = (b_idx as f32 * scale).round() as u8;
            let Oklab { l, .. } = srgb_to_oklab([rf, gf, bf].into());
            oklab_to_srgb(Oklab {
                l,
                a: colors[idx],
                b: colors[idx + 1],
            })
        } else {
            oklab_to_srgb(Oklab {
                l: colors[idx],
                a: colors[idx + 1],
                b: colors[idx + 2],
            })
        }
    }

    fn colors_to_lut(
        &self,
        colors: &[f32],
        size: usize,
        channels: usize,
        level: u8,
    ) -> Option<RgbImage> {
        let dim = (level as u32).pow(3);
        let scale = 255.0 / (size - 1) as f32;

        let mut buf = vec![0u8; (dim * dim * 3) as usize];

        // HALD CLUT order: blue (outer) -> green -> red (inner)
        for (pixel_idx, pixel) in buf.chunks_exact_mut(3).enumerate() {
            let (r_idx, g_idx, b_idx) = Self::pixel_to_rgb(pixel_idx, size);
            let idx = Self::cell_idx(r_idx, g_idx, b_idx, size) * channels;
            let rgb = Self::cell_to_rgb(colors, idx, self.preserve, scale, r_idx, g_idx, b_idx);
            pixel[0] = rgb.r;
            pixel[1] = rgb.g;
            pixel[2] = rgb.b;
        }

        RgbImage::from_raw(dim, dim, buf)
    }

    #[cfg(feature = "rayon")]
    fn par_colors_to_lut(
        &self,
        colors: &[f32],
        size: usize,
        channels: usize,
        level: u8,
    ) -> Option<RgbImage> {
        let dim = (level as u32).pow(3);
        let scale = 255.0 / (size - 1) as f32;
        let preserve = self.preserve;

        let mut buf = vec![0u8; (dim * dim * 3) as usize];

        buf.par_chunks_exact_mut(3)
            .enumerate()
            .for_each(|(pixel_idx, pixel)| {
                let (r_idx, g_idx, b_idx) = Self::pixel_to_rgb(pixel_idx, size);
                let idx = Self::cell_idx(r_idx, g_idx, b_idx, size) * channels;
                let rgb = Self::cell_to_rgb(colors, idx, preserve, scale, r_idx, g_idx, b_idx);
                pixel[0] = rgb.r;
                pixel[1] = rgb.g;
                pixel[2] = rgb.b;
            });

        RgbImage::from_raw(dim, dim, buf)
    }
}

#[inline(always)]
fn sq_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dl = a[0] - b[0];
    let da = a[1] - b[1];
    let db = a[2] - b[2];
    dl * dl + da * da + db * db
}

impl GenerateLut<'static> for GaussianBlurRemapper {
    fn generate_lut(&self, level: u8) -> RgbImage {
        self.generate_lut_inner(level, None)
            .expect("should not abort without signal")
    }

    #[cfg(feature = "rayon")]
    fn par_generate_lut(&self, level: u8) -> RgbImage {
        self.par_generate_lut_inner(level, None)
            .expect("should not abort without signal")
    }

    fn generate_lut_with_interrupt(&self, level: u8, abort: Arc<AtomicBool>) -> Option<RgbImage> {
        self.generate_lut_inner(level, Some(&abort))
    }

    #[cfg(feature = "rayon")]
    fn par_generate_lut_with_interrupt(
        &self,
        level: u8,
        abort: Arc<AtomicBool>,
    ) -> Option<RgbImage> {
        self.par_generate_lut_inner(level, Some(&abort))
    }
}
