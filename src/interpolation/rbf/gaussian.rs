use exoquant::ColorSpace;

use super::{RBFRemapper, RadialBasisFn};

/// The Gaussian function. Accepts a f64 euclide.
pub struct GaussianFn {
    euclide: f64,
}
impl RadialBasisFn for GaussianFn {
    fn radial_basis(&self, distance: f64) -> f64 {
        (-self.euclide * distance.powf(2.0)).exp()
    }
}

/// RBF interpolating remapper using the [`GaussianFn`] between n nearest colors.
///
/// Lower euclide values will have more of a gradient between colors,
/// but with more washed out results. Higher euclide values will keep
/// the colors more true, but with less gradient between them. Lowering
/// the number of nearest neighbors can also mitigate washout, but
/// may increase banding when using the LUT for corrections.
pub type GaussianRemapper<'a, CS> = RBFRemapper<'a, GaussianFn, CS>;
impl<'a, CS: ColorSpace + Sync> GaussianRemapper<'a, CS> {
    pub fn new(palette: &'a [[u8; 3]], euclide: f64, nearest: usize, colorspace: CS) -> Self {
        RBFRemapper::with_function(palette, GaussianFn { euclide }, nearest, colorspace)
    }
}
