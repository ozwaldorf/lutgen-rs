use std::f64;

use kiddo::{distance_metric::DistanceMetric, NearestNeighbour, SquaredEuclidean};

use super::{ColorTree, InterpolatedRemapper};
use crate::GenerateLut;

pub trait RadialBasisFn: Sync {
    fn radial_basis(&self, distance: f64) -> f64;
}

pub struct RBFRemapper<F: RadialBasisFn> {
    rbf: F,
    tree: Option<(usize, ColorTree)>,
    palette: Vec<[f64; 3]>,
    lum_factor: f64,
    preserve_lum: bool,
}

impl<'a, F: RadialBasisFn> GenerateLut<'a> for RBFRemapper<F> {}
impl<'a, F: RadialBasisFn> RBFRemapper<F> {
    pub fn with_function(
        palette: &'a [[u8; 3]],
        rbf: F,
        nearest: usize,
        lum_factor: f64,
        preserve_lum: bool,
    ) -> Self {
        let palette: Vec<_> = palette
            .iter()
            .map(|raw| {
                let oklab = oklab::srgb_to_oklab((*raw).into());
                [oklab.l as f64 * lum_factor, oklab.a as f64, oklab.b as f64]
            })
            .collect();

        let tree = if nearest > 0 && palette.len() < nearest {
            let mut tree = ColorTree::with_capacity(palette.len());
            for (i, color) in palette.iter().enumerate() {
                tree.add(color, i as u32);
            }

            Some((nearest, tree))
        } else {
            None
        };

        Self {
            rbf,
            tree,
            palette,
            lum_factor,
            preserve_lum,
        }
    }
}

impl<'a, F: RadialBasisFn> InterpolatedRemapper<'a> for RBFRemapper<F> {
    fn remap_pixel(&self, pixel: &mut image::Rgb<u8>) {
        let raw_color = &mut pixel.0;
        let color = oklab::srgb_to_oklab((*raw_color).into());
        let color = [
            color.l as f64 * self.lum_factor,
            color.a as f64,
            color.b as f64,
        ];

        if self.palette.contains(&color) {
            return;
        }

        let mut numerator = [0.0; 3];
        let mut denominator = 0.0;

        match &self.tree {
            None => {
                for p_color in self.palette.iter() {
                    let distance = SquaredEuclidean::dist(&color, p_color);
                    let weight = self.rbf.radial_basis(distance);

                    numerator[0] += p_color[0] * weight;
                    numerator[1] += p_color[1] * weight;
                    numerator[2] += p_color[2] * weight;
                    denominator += weight;
                }
            },
            Some((nearest, tree)) => {
                for NearestNeighbour { item, distance } in
                    tree.nearest_n::<SquaredEuclidean>(&color, *nearest)
                {
                    let weight = self.rbf.radial_basis(distance);
                    let p_color = self.palette[item as usize];

                    numerator[0] += p_color[0] * weight;
                    numerator[1] += p_color[1] * weight;
                    numerator[2] += p_color[2] * weight;
                    denominator += weight;
                }
            },
        }

        *raw_color = oklab::oklab_to_srgb(oklab::Oklab {
            l: if self.preserve_lum {
                (color[0] / self.lum_factor) as f32
            } else {
                (numerator[0] / denominator / self.lum_factor) as f32
            },
            a: (numerator[1] / denominator) as f32,
            b: (numerator[2] / denominator) as f32,
        })
        .into();
    }
}

#[macro_export]
macro_rules! impl_rbf {
    (
        $($doc:expr,)?
        $name:ident<$fn_name:ident>,
        $fn:expr
        $(, { $($param:ident: $param_ty:ty),* })?

    ) => {
        $(#[doc = $doc])?
        pub type $name = RBFRemapper<$fn_name>;
        impl $name {
            pub fn new(
                palette: &[[u8; 3]],
                $($($param: $param_ty,)*)?
                nearest: usize,
                lum_factor: f64,
                preserve_lum: bool
            ) -> Self {
                RBFRemapper::with_function(
                    palette,
                    $fn_name { $($($param),*)? },
                    nearest,
                    lum_factor,
                    preserve_lum
                )
            }
        }

        pub struct $fn_name { $($($param: $param_ty,)*)? }
        impl RadialBasisFn for $fn_name {
            fn radial_basis(&self, distance: f64) -> f64 {
                let rbf: fn(&Self, f64) -> f64 = $fn;
                rbf(self, distance)
            }
        }
    };
}

impl_rbf!(
    "RBF remapper using a linear function on N nearest neighbors. 

It's recommended to use a low number of neighbors for this method, otherwise the results will be extremely washed out.",
    LinearRemapper<LinearFn>,
    |_, d| d
);

impl_rbf!(
    "Shepards Method, aka an RBF remapper using the inverse distance function on N nearest neighbors.

Lower power values will result in a longer gradient between the colors, but with more washed out results.
Lowering the number of nearest colors can also mitigate washout, but may increase banding when using the LUT for corrections.",
    ShepardRemapper<InverseDistanceFn>,
    |s, d| { 1.0 / d.sqrt().powf(s.power) },
    { power: f64 }
);

impl_rbf!(
    "RBF remapper using the Gaussian function on N nearest neighbors.

Lower shape values will have more of a gradient between colors, but with more washed out results.
Higher shape values will keep the colors more true, but with less gradient between them. 
Lowering the number of nearest neighbors can also mitigate washout, but may increase banding when using the LUT for corrections.",
    GaussianRemapper<GaussianFn>,
    |s, d| (-s.shape * d).exp(),
    { shape: f64 }
);
