//! Gaussian blur LUT remapping in OKLab color space.
//!
//! Creates a nearest-neighbor LUT with OKLab colors,
//! then applies separable Gaussian blur directly on the color values.

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
pub struct GaussianBlurRemapper<'a> {
    #[allow(dead_code)]
    palette: &'a [[u8; 3]],
    palette_oklab: Vec<[f32; 3]>,
    radius: f32,
    lum_factor: f32,
    preserve: bool,
}

impl<'a> GaussianBlurRemapper<'a> {
    #[inline]
    pub fn new(palette: &'a [[u8; 3]], radius: f64, lum_factor: f64, preserve: bool) -> Self {
        let lum_factor = lum_factor as f32;

        let palette_oklab: Vec<[f32; 3]> = palette
            .iter()
            .map(|&color| {
                let Oklab { l, a, b } = srgb_to_oklab(color.into());
                [l * lum_factor, a, b]
            })
            .collect();

        Self {
            palette,
            palette_oklab,
            radius: radius as f32,
            lum_factor,
            preserve,
        }
    }

    #[inline(always)]
    fn find_nearest(&self, color: [f32; 3]) -> usize {
        let mut best_idx = 0;
        let mut best_dist = f32::MAX;
        for (i, target) in self.palette_oklab.iter().enumerate() {
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

    /// Apply a 1D Gaussian blur along a single axis of the 3D LUT.
    ///
    /// `axis_idx` maps `(r, g, b, kernel_offset)` to the source cell index,
    /// where `kernel_offset` is a signed offset along the axis being blurred.
    fn blur_axis(
        colors: &[f32],
        colors_next: &mut [f32],
        size: usize,
        channels: usize,
        kernel: &[f32],
        axis_idx: impl Fn(usize, usize, usize, i32) -> usize,
    ) {
        let half = (kernel.len() / 2) as i32;
        for r in 0..size {
            for g in 0..size {
                for b in 0..size {
                    let idx_out = Self::cell_idx(r, g, b, size) * channels;
                    for c in 0..channels {
                        let mut sum = 0.0f32;
                        for (ki, &kw) in kernel.iter().enumerate() {
                            let idx_in = axis_idx(r, g, b, ki as i32 - half) * channels + c;
                            sum += kw * colors[idx_in];
                        }
                        colors_next[idx_out + c] = sum;
                    }
                }
            }
        }
    }

    #[cfg(feature = "rayon")]
    fn par_blur_axis(
        colors: &[f32],
        colors_next: &mut [f32],
        size: usize,
        channels: usize,
        kernel: &[f32],
        axis_idx: impl Fn(usize, usize, usize, i32) -> usize + Sync,
    ) {
        let half = (kernel.len() / 2) as i32;
        colors_next
            .par_chunks_mut(channels)
            .enumerate()
            .for_each(|(idx, out)| {
                let b = idx % size;
                let g = (idx / size) % size;
                let r = idx / (size * size);

                for (c, out_c) in out.iter_mut().enumerate().take(channels) {
                    let mut sum = 0.0f32;
                    for (ki, &kw) in kernel.iter().enumerate() {
                        let idx_in = axis_idx(r, g, b, ki as i32 - half) * channels + c;
                        sum += kw * colors[idx_in];
                    }
                    *out_c = sum;
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

        for r in 0..size {
            let rf = (r as f32 * scale).round() as u8;
            for g in 0..size {
                let gf = (g as f32 * scale).round() as u8;
                for b in 0..size {
                    let bf = (b as f32 * scale).round() as u8;
                    let Oklab { l, a, b: ob } = srgb_to_oklab([rf, gf, bf].into());
                    let nearest = self.find_nearest([l * self.lum_factor, a, ob]);
                    let target = &self.palette_oklab[nearest];

                    if self.preserve {
                        // Only store a, b from target (L comes from input at output time)
                        colors.push(target[1]); // a
                        colors.push(target[2]); // b
                    } else {
                        colors.push(target[0] / self.lum_factor); // restore original L
                        colors.push(target[1]);
                        colors.push(target[2]);
                    }
                }
            }
        }

        let mut colors_next = vec![0.0f32; n_cells * channels];
        let kernel = self.build_kernel();
        let max = size as i32 - 1;
        let clamp = |v: usize, off: i32| (v as i32 + off).clamp(0, max) as usize;

        // Blur along R axis
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::blur_axis(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            |r, g, b, off| Self::cell_idx(clamp(r, off), g, b, size),
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Blur along G axis
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::blur_axis(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            |r, g, b, off| Self::cell_idx(r, clamp(g, off), b, size),
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Blur along B axis
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::blur_axis(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            |r, g, b, off| Self::cell_idx(r, g, clamp(b, off), size),
        );
        std::mem::swap(&mut colors, &mut colors_next);

        self.colors_to_lut(&colors, size, channels, level)
    }

    #[cfg(feature = "rayon")]
    fn par_generate_lut_inner(&self, level: u8, abort: Option<&AtomicBool>) -> Option<RgbImage> {
        let size = (level as usize).pow(2);
        let n_cells = size * size * size;
        let scale = 255.0 / (size - 1) as f32;

        let channels = if self.preserve { 2 } else { 3 };

        // Build NN LUT with OKLab colors in parallel
        let mut colors: Vec<f32> = if self.preserve {
            (0..n_cells)
                .into_par_iter()
                .flat_map_iter(|idx| {
                    let b = idx % size;
                    let g = (idx / size) % size;
                    let r = idx / (size * size);

                    let rf = (r as f32 * scale).round() as u8;
                    let gf = (g as f32 * scale).round() as u8;
                    let bf = (b as f32 * scale).round() as u8;
                    let Oklab { l, a, b: ob } = srgb_to_oklab([rf, gf, bf].into());
                    let nearest = self.find_nearest([l * self.lum_factor, a, ob]);
                    let target = &self.palette_oklab[nearest];

                    [target[1], target[2]]
                })
                .collect()
        } else {
            (0..n_cells)
                .into_par_iter()
                .flat_map_iter(|idx| {
                    let b = idx % size;
                    let g = (idx / size) % size;
                    let r = idx / (size * size);

                    let rf = (r as f32 * scale).round() as u8;
                    let gf = (g as f32 * scale).round() as u8;
                    let bf = (b as f32 * scale).round() as u8;
                    let Oklab { l, a, b: ob } = srgb_to_oklab([rf, gf, bf].into());
                    let nearest = self.find_nearest([l * self.lum_factor, a, ob]);
                    let target = &self.palette_oklab[nearest];

                    [target[0] / self.lum_factor, target[1], target[2]]
                })
                .collect()
        };

        let mut colors_next = vec![0.0f32; n_cells * channels];
        let kernel = self.build_kernel();
        let max = size as i32 - 1;
        let clamp = |v: usize, off: i32| (v as i32 + off).clamp(0, max) as usize;

        // Blur along R axis
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::par_blur_axis(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            |r, g, b, off| Self::cell_idx(clamp(r, off), g, b, size),
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Blur along G axis
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::par_blur_axis(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            |r, g, b, off| Self::cell_idx(r, clamp(g, off), b, size),
        );
        std::mem::swap(&mut colors, &mut colors_next);

        // Blur along B axis
        if abort.is_some_and(|a| a.load(std::sync::atomic::Ordering::Relaxed)) {
            return None;
        }
        Self::par_blur_axis(
            &colors,
            &mut colors_next,
            size,
            channels,
            &kernel,
            |r, g, b, off| Self::cell_idx(r, g, clamp(b, off), size),
        );
        std::mem::swap(&mut colors, &mut colors_next);

        self.colors_to_lut(&colors, size, channels, level)
    }

    fn colors_to_lut(
        &self,
        colors: &[f32],
        size: usize,
        channels: usize,
        level: u8,
    ) -> Option<RgbImage> {
        let level = level as u32;
        let width = level * level * level;
        let height = level * level * level;
        let scale = 255.0 / (size - 1) as f32;

        let mut buf = vec![0u8; (width * height * 3) as usize];

        // HALD CLUT order: blue (outer) -> green -> red (inner)
        let mut pixel_idx = 0usize;
        for b_idx in 0..size {
            for g_idx in 0..size {
                for r_idx in 0..size {
                    let idx = Self::cell_idx(r_idx, g_idx, b_idx, size) * channels;
                    let out_idx = pixel_idx * 3;

                    let rgb = if self.preserve {
                        // Reconstruct L from input, use blurred a, b
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
                    };

                    buf[out_idx] = rgb.r;
                    buf[out_idx + 1] = rgb.g;
                    buf[out_idx + 2] = rgb.b;
                    pixel_idx += 1;
                }
            }
        }

        RgbImage::from_raw(width, height, buf)
    }
}

#[inline(always)]
fn sq_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dl = a[0] - b[0];
    let da = a[1] - b[1];
    let db = a[2] - b[2];
    dl * dl + da * da + db * db
}

impl<'a> GenerateLut<'a> for GaussianBlurRemapper<'a> {
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
