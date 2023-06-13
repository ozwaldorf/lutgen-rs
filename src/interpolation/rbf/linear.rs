use exoquant::ColorSpace;

use super::{RBFRemapper, RadialBasisFn};

/// Linear function (`1 / distance`)
pub struct LinearFn;
impl RadialBasisFn for LinearFn {
    fn radial_basis(&self, distance: f64) -> f64 {
        1.0 / distance
    }
}

/// RBF interpolation using a [`LinearFn`] between n nearest colors.
///
/// Higher numbers of neighbors will produce smoother, but more washed out results.
pub type LinearRemapper<'a, CS> = RBFRemapper<'a, LinearFn, CS>;
impl<'a, CS: ColorSpace + Sync> LinearRemapper<'a, CS> {
    pub fn new(palette: &'a [[u8; 3]], nearest: usize, colorspace: CS) -> Self {
        RBFRemapper::with_function(palette, LinearFn, nearest, colorspace)
    }
}
