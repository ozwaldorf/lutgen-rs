use exoquant::ColorSpace;

use super::{RBFParams, RBFRemapper, RadialBasisFn};

/// The Gaussian function. Accepts a f64 euclide.

pub struct GaussianFn;
impl RadialBasisFn for GaussianFn {
    type Params = f64;
    fn radial_basis(euclide: f64, distance: f64) -> f64 {
        (-euclide * distance.powf(2.0)).exp()
    }
}

/// RBF interpolating remapper using the [`GaussianFn`]
///
/// Lower euclide values will have more of a gradient between colors,
/// but with more washed out results. Higher euclide values will keep
/// the colors more true, but with less gradient between them. Lowering
/// the number of nearest neighbors can also mitigate washout, but
/// may increase banding when using the LUT for corrections.
pub type GaussianRemapper<'a, CS> = RBFRemapper<'a, GaussianFn, CS>;
pub struct GaussianParams;
impl GaussianParams {
    pub fn new<CS: ColorSpace + Sync>(
        euclide: f64,
        nearest: usize,
        colorspace: CS,
    ) -> RBFParams<GaussianFn, CS> {
        RBFParams::new(euclide, nearest, colorspace)
    }
}
