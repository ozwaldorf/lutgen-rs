use super::{RBFRemapper, RadialBasisFn};

/// Inverse Distance Funciton. Accepts a f64 power parameter.
pub struct InverseDistanceFn {
    power: f64,
}
impl RadialBasisFn for InverseDistanceFn {
    fn radial_basis(&self, distance: f64) -> f64 {
        1.0 / distance.sqrt().powf(self.power)
    }
}

/// Shepard's method, a form of RBF interpolation using the [`InverseDistanceFn`]
/// between n nearest colors.
///
/// Lower power values will result in a longer gradient between the colors, but with
/// more washed out results. Lowering the number of nearest colors can mitigate
/// this, but may increase banding when using the final LUT for corrections.
pub type ShepardRemapper = RBFRemapper<InverseDistanceFn>;
impl ShepardRemapper {
    pub fn new(palette: &[[u8; 3]], power: f64, nearest: usize, lum_factor: f64) -> Self {
        RBFRemapper::with_function(palette, InverseDistanceFn { power }, nearest, lum_factor)
    }
}
