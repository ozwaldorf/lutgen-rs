use super::{RBFRemapper, RadialBasisFn};

/// The Gaussian function. Accepts a f64 euclide.
pub struct GaussianFn {
    shape: f64,
}
impl RadialBasisFn for GaussianFn {
    fn radial_basis(&self, distance: f64) -> f64 {
        (-self.shape * distance.powf(2.0)).exp()
    }
}

/// RBF interpolating remapper using the [`GaussianFn`] between n nearest colors.
///
/// Lower euclide values will have more of a gradient between colors,
/// but with more washed out results. Higher euclide values will keep
/// the colors more true, but with less gradient between them. Lowering
/// the number of nearest neighbors can also mitigate washout, but
/// may increase banding when using the LUT for corrections.
pub type GaussianRemapper = RBFRemapper<GaussianFn>;
impl GaussianRemapper {
    pub fn new(palette: &[[u8; 3]], shape: f64, nearest: usize, lum_factor: f64) -> Self {
        RBFRemapper::with_function(palette, GaussianFn { shape }, nearest, lum_factor)
    }
}
