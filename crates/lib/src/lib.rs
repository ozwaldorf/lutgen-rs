//! # Lutgen (Library)
//!
//! > For documentation on the cli application, see the [repository](https://github.com/ozwaldorf/lutgen-rs#usage).
//!
//! ## Default Features
//!
//! By default, the `bin` feature and dependencies are enabled.
//! When used as a library, it's recommended to use `default-features = false` to minimize the
//! dependency tree and build times.
//!
//! ## Examples
//!
//! #### Generating a LUT
//!
//! ```rust
//! use lutgen::interpolation::{GaussianRemapper, GaussianSamplingRemapper};
//! use lutgen::GenerateLut;
//! use lutgen_palettes::Palette;
//!
//! // Get a premade palette
//! let palette = Palette::Carburetor.get();
//!
//! // Setup the fast Gaussian RBF algorithm
//! let (shape, nearest, lum_factor, preserve) = (128.0, 0, 1.0, false);
//! let remapper = GaussianRemapper::new(&palette, shape, nearest, lum_factor, preserve);
//!
//! // Generate and remap a HALD:8 for the provided palette
//! let hald_clut = remapper.par_generate_lut(8);
//!
//! // hald_clut.save("output.png").unwrap();
//!
//! // Setup another palette to interpolate from, with custom colors
//! let palette = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
//!
//! // Setup the slower Gaussian Sampling algorithm
//! let (mean, std_dev, iters, lum_factor, seed, preserve) = (0.0, 20.0, 512, 1.0, 420, true);
//! let remapper =
//!     GaussianSamplingRemapper::new(&palette, mean, std_dev, iters, lum_factor, seed, preserve);
//!
//! // Generate and remap a HALD:4 for the provided palette
//! let hald_clut = remapper.par_generate_lut(4);
//!
//! // hald_clut.save("output.png").unwrap();
//! ```
//!
//! #### Applying a LUT
//!
//! ```rust
//! use image::buffer::ConvertBuffer;
//! use image::open;
//! use lutgen::identity::correct_image;
//! use lutgen::interpolation::GaussianRemapper;
//! use lutgen::GenerateLut;
//! use lutgen_palettes::Palette;
//!
//! // Generate a hald clut
//! let palette = Palette::GruvboxDark.get();
//! let remapper = GaussianRemapper::new(&palette, 96.0, 0, 1.0, false);
//! let hald_clut = remapper.par_generate_lut(8);
//!
//! // Save the LUT for later
//! hald_clut
//!     .save("../../docs/assets/gruvbox-dark-hald-clut.png")
//!     .unwrap();
//!
//! // Open an image to correct
//! let mut external_image = open("../../docs/assets/example-image.jpg")
//!     .unwrap()
//!     .to_rgba8();
//!
//! // Correct the image using the hald clut we generated
//! correct_image(&mut external_image, &hald_clut);
//!
//! // Save the edited image
//! let rgbimage: image::RgbImage = external_image.convert();
//! rgbimage.save("../../docs/assets/gruvbox-dark.jpg").unwrap()
//! ```
//!
//! #### Remapping an image directly
//!
//! > Note: While the remappers *can* be used directly on any image, it's much
//! > faster to remap a LUT and correct an image with that.
//!
//! ```rust
//! use image::buffer::ConvertBuffer;
//! use lutgen::interpolation::{GaussianRemapper, InterpolatedRemapper};
//! use lutgen::GenerateLut;
//!
//! // Setup the palette to interpolate from
//! let palette = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
//!
//! // Setup a remapper
//! let (shape, nearest, lum_factor, preserve) = (96.0, 0, 1.0, false);
//! let remapper = GaussianRemapper::new(&palette, shape, nearest, lum_factor, preserve);
//!
//! // Generate an image (generally an identity lut to use on other images)
//! let mut hald_clut = lutgen::identity::generate(8).convert();
//!
//! // Remap the image
//! remapper.par_remap_image(&mut hald_clut);
//!
//! // hald_clut.save("output.png").unwrap();
//! ```

use std::ops::Not;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use image::buffer::ConvertBuffer;
use interpolation::InterpolatedRemapper;

pub mod identity;
pub mod interpolation;

/// Core image type (Rgba8)
pub use image::{RgbImage, RgbaImage};

pub trait GenerateLut<'a>: InterpolatedRemapper<'a> {
    /// Helper method to generate a lut using an [`InterpolatedRemapper`].
    fn generate_lut(&self, level: u8) -> RgbImage {
        let mut identity = identity::generate(level).convert();
        self.remap_image(&mut identity);
        identity.convert()
    }

    /// Rayon version. Helper method to generate a lut using an [`InterpolatedRemapper`].
    fn par_generate_lut(&self, level: u8) -> RgbImage {
        let mut identity = identity::generate(level).convert();
        self.par_remap_image(&mut identity);
        identity.convert()
    }

    /// Same as [`GenerateLut::generate_lut`], but aborts and returns nothing if the given boolean
    /// is true.
    fn generate_lut_with_interrupt(&self, level: u8, abort: Arc<AtomicBool>) -> Option<RgbImage> {
        let mut identity = identity::generate(level).convert();
        self.remap_image_with_interrupt(&mut identity, abort.clone());
        abort
            .load(std::sync::atomic::Ordering::Relaxed)
            .not()
            .then_some(identity.convert())
    }

    /// Rayon version. Same as [`GenerateLut::generate_lut`], but aborts and returns nothing if the
    /// given boolean is true.
    fn par_generate_lut_with_interrupt(
        &self,
        level: u8,
        abort: Arc<AtomicBool>,
    ) -> Option<RgbImage> {
        let mut identity = identity::generate(level).convert();
        self.par_remap_image_with_interrupt(&mut identity, abort.clone());
        abort
            .load(std::sync::atomic::Ordering::Relaxed)
            .not()
            .then_some(identity.convert())
    }
}
