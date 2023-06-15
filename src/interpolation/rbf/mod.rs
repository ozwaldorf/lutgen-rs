use std::f64;

use kiddo::float::{kdtree::KdTree, neighbour::Neighbour};

use super::InterpolatedRemapper;
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
}

type ColorTree = KdTree<f64, u16, 3, 4, u16>;

impl<'a, F: RadialBasisFn> GenerateLut<'a> for RBFRemapper<F> {}
impl<'a, F: RadialBasisFn> RBFRemapper<F> {
    fn with_function(palette: &'a [[u8; 3]], rbf: F, nearest: usize) -> Self {
        let palette: Vec<_> = palette
            .iter()
            .map(|raw| {
                let oklab = oklab::srgb_to_oklab((*raw).into());
                [oklab.l as f64, oklab.a as f64, oklab.b as f64]
            })
            .collect();

        let mut tree = None;

        if nearest > 0 && palette.len() < nearest {
            let mut kdtree = KdTree::with_capacity(palette.len());
            for (i, raw) in palette.iter().enumerate() {
                kdtree.add(raw, i as u16);
            }
            tree = Some((nearest, kdtree));
        };

        Self { rbf, tree, palette }
    }
}

impl<'a, F: RadialBasisFn> InterpolatedRemapper<'a> for RBFRemapper<F> {
    fn remap_pixel(&self, pixel: &mut image::Rgb<u8>) {
        let raw_color = &mut pixel.0;
        let color = oklab::srgb_to_oklab((*raw_color).into());
        let color = [color.l as f64, color.a as f64, color.b as f64];

        if self.palette.contains(&color) {
            return;
        }

        let mut numerator = [0.0; 3];
        let mut denominator = 0.0;

        match &self.tree {
            None => {
                for p_color in self.palette.iter() {
                    let dl = color[0] - p_color[0];
                    let da = color[1] - p_color[1];
                    let db = color[2] - p_color[2];
                    let distance = (dl * dl + da * da + db * db).sqrt();

                    let weight = self.rbf.radial_basis(distance);

                    numerator[0] += p_color[0] * weight;
                    numerator[1] += p_color[1] * weight;
                    numerator[2] += p_color[2] * weight;
                    denominator += weight;
                }
            },
            Some((nearest, tree)) => {
                for Neighbour { item, distance } in
                    tree.nearest_n(&color, *nearest, &kiddo::distance::squared_euclidean)
                {
                    let distance = distance.sqrt(); // kiddo returns distance^2

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
            l: (numerator[0] / denominator) as f32,
            a: (numerator[1] / denominator) as f32,
            b: (numerator[2] / denominator) as f32,
        })
        .into();
    }
}
