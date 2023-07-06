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
pub type LinearRemapper = RBFRemapper<LinearFn>;
impl LinearRemapper {
    pub fn new(palette: &[[u8; 3]], nearest: usize, lum_factor: f64) -> Self {
        RBFRemapper::with_function(palette, LinearFn, nearest, lum_factor)
    }
}
