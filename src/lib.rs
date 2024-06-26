//! # Lutgen (Library)
//!
//! > Note: For documentation on the cli application, see the [readme](https://github.com/ozwaldorf/lutgen-rs).
//!
//! ## Default Features
//!
//! By default, the `bin` feature and dependencies are enabled.
//! When used as a library, it's recommended to use `default-features = false` to minimize the
//! dependency tree and build times.
//!
//! ## Generating a LUT
//!
//! ```rust
//! use lutgen::{
//!     interpolation::{GaussianRemapper, GaussianSamplingRemapper},
//!     GenerateLut,
//! };
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
//! let hald_clut = remapper.generate_lut(8);
//!
//! // hald_clut.save("output.png").unwrap();
//!
//! // Setup another palette to interpolate from, with custom colors
//! let palette = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
//!
//! // Setup the slower Gaussian Sampling algorithm
//! let (mean, std_dev, iters, lum_factor, seed) = (0.0, 20.0, 512, 1.0, 420);
//! let remapper = GaussianSamplingRemapper::new(&palette, mean, std_dev, iters, lum_factor, seed);
//!
//! // Generate and remap a HALD:4 for the provided palette
//! let hald_clut = remapper.generate_lut(4);
//!
//! // hald_clut.save("output.png").unwrap();
//! ```
//!
//! ## Applying a LUT
//!
//! ```rust
//! use image::open;
//! use lutgen::{identity::correct_image, interpolation::GaussianRemapper, GenerateLut};
//! use lutgen_palettes::Palette;
//!
//! // Generate a hald clut
//! let palette = Palette::Carburetor.get();
//! let remapper = GaussianRemapper::new(&palette, 96.0, 0, 1.0, false);
//! let hald_clut = remapper.generate_lut(8);
//!
//! // Save the LUT for later
//! hald_clut.save("docs/carburetor-hald-clut.png").unwrap();
//!
//! // Open an image to correct
//! let mut external_image = open("docs/example-image.jpg").unwrap().to_rgb8();
//!
//! // Correct the image using the hald clut we generated
//! correct_image(&mut external_image, &hald_clut);
//!
//! // Save the edited image
//! external_image.save("docs/catppuccin-mocha.jpg").unwrap()
//! ```
//!
//! ## Remapping an image directly
//!
//! > Note: While the remappers *can* be used directly on any image, it's much
//! > faster to remap a LUT and correct an image with that.
//!
//! ```rust
//! use lutgen::{
//!     interpolation::{GaussianRemapper, InterpolatedRemapper},
//!     GenerateLut,
//! };
//!
//! // Setup the palette to interpolate from
//! let palette = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
//!
//! // Setup a remapper
//! let (shape, nearest, lum_factor, preserve) = (96.0, 0, 1.0, false);
//! let remapper = GaussianRemapper::new(&palette, shape, nearest, lum_factor, preserve);
//!
//! // Generate an image (generally an identity lut to use on other images)
//! let mut hald_clut = lutgen::identity::generate(8);
//!
//! // Remap the image
//! remapper.remap_image(&mut hald_clut);
//!
//! // hald_clut.save("output.png").unwrap();
//! ```

use image::{ImageBuffer, Rgb};
use interpolation::InterpolatedRemapper;

pub mod identity;
pub mod interpolation;

/// Core image type (Rgb8)
pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

pub trait GenerateLut<'a>: InterpolatedRemapper<'a> {
    /// Helper method to generate a lut using an [`InterpolatedRemapper`].
    fn generate_lut(&self, level: u8) -> Image {
        let mut identity = identity::generate(level);
        self.remap_image(&mut identity);
        identity
    }
}
