use exoquant::ColorSpace;

use super::{RBFParams, RBFRemapper, RadialBasisFn};

/// Inverse Distance Funciton. Accepts a f64 power parameter.
pub struct InverseDistanceFn;
impl RadialBasisFn for InverseDistanceFn {
    type Params = f64;
    fn radial_basis(power: f64, distance: f64) -> f64 {
        1.0 / distance.powf(power)
    }
}

/// Shepard's method for interpolation. A form of RBF interpolation using
/// the inverse distance function.
///
/// Lower power values will result in more gradient between
/// the colors, but with more washed out results. Lowering the
/// number of nearest colors can mitigate this, but may increase
/// banding when using the LUT for corrections.
pub type ShepardRemapper<'a, CS> = RBFRemapper<'a, InverseDistanceFn, CS>;
pub struct ShepardParams;
impl ShepardParams {
    pub fn new<CS: ColorSpace + Sync>(
        power: f64,
        num_nearest: usize,
        colorspace: CS,
    ) -> RBFParams<InverseDistanceFn, CS> {
        RBFParams::new(power, num_nearest, colorspace)
    }
}
