use exoquant::ColorSpace;

use super::{RBFRemapper, RadialBasisFn};

/// Inverse Distance Funciton. Accepts a f64 power parameter.
pub struct InverseDistanceFn {
    power: f64,
}
impl RadialBasisFn for InverseDistanceFn {
    fn radial_basis(&self, distance: f64) -> f64 {
        1.0 / distance.powf(self.power)
    }
}

/// Shepard's method, a form of RBF interpolation using the [`InverseDistanceFn`]
/// between n nearest colors.
///
/// Lower power values will result in a longer gradient between the colors, but with
/// more washed out results. Lowering the number of nearest colors can mitigate
/// this, but may increase banding when using the final LUT for corrections.
pub type ShepardRemapper<'a, CS> = RBFRemapper<'a, InverseDistanceFn, CS>;
impl<'a, CS: ColorSpace + Sync> ShepardRemapper<'a, CS> {
    pub fn new(palette: &'a [[u8; 3]], power: f64, nearest: usize, colorspace: CS) -> Self {
        RBFRemapper::with_function(palette, InverseDistanceFn { power }, nearest, colorspace)
    }
}
