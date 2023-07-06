use std::f64;

use kiddo::float::{kdtree::KdTree, neighbour::Neighbour};

use super::{euclidean, ColorTree, InterpolatedRemapper};
use crate::GenerateLut;

pub mod gaussian;
pub mod linear;
pub mod shepard;

pub trait RadialBasisFn: Sync {
    fn radial_basis(&self, distance: f64) -> f64;
}

pub struct RBFRemapper<F: RadialBasisFn> {
    rbf: F,
    tree: Option<(usize, ColorTree)>,
    palette: Vec<[f64; 3]>,
    lum_factor: f64,
}

impl<'a, F: RadialBasisFn> GenerateLut<'a> for RBFRemapper<F> {}
impl<'a, F: RadialBasisFn> RBFRemapper<F> {
    fn with_function(palette: &'a [[u8; 3]], rbf: F, nearest: usize, lum_factor: f64) -> Self {
        let palette: Vec<_> = palette
            .iter()
            .map(|raw| {
                let oklab = oklab::srgb_to_oklab((*raw).into());
                [oklab.l as f64 * lum_factor, oklab.a as f64, oklab.b as f64]
            })
            .collect();

        let mut tree = None;

        if nearest > 0 && palette.len() < nearest {
            let mut kdtree = KdTree::with_capacity(palette.len());
            for (i, color) in palette.iter().enumerate() {
                kdtree.add(color, i as u16);
            }
            tree = Some((nearest, kdtree));
        };

        Self {
            rbf,
            tree,
            palette,
            lum_factor,
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
                    let distance = euclidean(&color, p_color);
                    let weight = self.rbf.radial_basis(distance);

                    numerator[0] += p_color[0] * weight;
                    numerator[1] += p_color[1] * weight;
                    numerator[2] += p_color[2] * weight;
                    denominator += weight;
                }
            },
            Some((nearest, tree)) => {
                for Neighbour { item, distance } in tree.nearest_n(&color, *nearest, &euclidean) {
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
            l: (numerator[0] / denominator / self.lum_factor) as f32,
            a: (numerator[1] / denominator) as f32,
            b: (numerator[2] / denominator) as f32,
        })
        .into();
    }
}
