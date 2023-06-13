use exoquant::ColorSpace;

use super::{RBFParams, RBFRemapper, RadialBasisFn};

/// Linear function (`1 / distance`)
pub struct LinearFn;
impl RadialBasisFn for LinearFn {
    type Params = ();
    fn radial_basis(_: Self::Params, distance: f64) -> f64 {
        1.0 / distance
    }
}

/// Linear interpolation between n nearest colors.
///
/// Higher numbers of neighbors will produce smoother,
/// but more washed out results.
pub type LinearRemapper<'a, CS> = RBFRemapper<'a, LinearFn, CS>;
pub struct LinearParams;
impl LinearParams {
    pub fn new<CS: ColorSpace + Sync>(
        num_nearest: usize,
        colorspace: CS,
    ) -> RBFParams<LinearFn, CS> {
        RBFParams::new((), num_nearest, colorspace)
    }
}
