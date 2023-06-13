use std::{f64, marker::PhantomData};

use exoquant::{Color, ColorSpace, Colorf};
use kiddo::float::{kdtree::KdTree, neighbour::Neighbour};

use super::InterpolatedRemapper;

pub mod gaussian;
pub mod linear;
pub mod shepard;

pub trait RadialBasisFn: Sync {
    type Params: Copy + Sync;
    fn radial_basis(p: Self::Params, distance: f64) -> f64;
}

pub struct RBFRemapper<'a, F: RadialBasisFn, CS: ColorSpace + Sync> {
    rbf: PhantomData<F>,
    rbf_params: F::Params,
    ref_palette: TreeOrVec,
    true_palette: &'a [[u8; 3]],
    colorspace: CS,
}

type ColorTree = KdTree<f64, u16, 3, 4, u16>;
enum TreeOrVec {
    Tree(usize, ColorTree),
    Vec(Vec<Colorf>),
}

pub struct RBFParams<F: RadialBasisFn, CS: ColorSpace> {
    p: PhantomData<F>,
    pub params: F::Params,
    pub nearest: usize,
    pub colorspace: CS,
}

impl<'a, F: RadialBasisFn, CS: ColorSpace + Sync> RBFParams<F, CS> {
    pub fn new(params: F::Params, nearest: usize, colorspace: CS) -> Self {
        Self {
            p: PhantomData::<F>,
            params,
            nearest,
            colorspace,
        }
    }
}

impl<'a, F: RadialBasisFn, CS: ColorSpace + Sync> InterpolatedRemapper<'a>
    for RBFRemapper<'a, F, CS>
{
    type Params = RBFParams<F, CS>;

    fn new(palette: &'a [[u8; 3]], params: Self::Params) -> Self {
        let true_palette = palette;
        let ref_palette = if params.nearest == 0 || palette.len() <= params.nearest {
            TreeOrVec::Vec(
                palette
                    .iter()
                    .map(|raw| {
                        params
                            .colorspace
                            .to_float(Color::new(raw[0], raw[1], raw[2], 255))
                    })
                    .collect(),
            )
        } else {
            let mut kdtree = KdTree::with_capacity(palette.len());
            for (i, &raw) in palette.iter().enumerate() {
                let c = params
                    .colorspace
                    .to_float(Color::new(raw[0], raw[1], raw[2], 255));
                kdtree.add(&[c.r, c.g, c.b], i as u16);
            }
            TreeOrVec::Tree(params.nearest, kdtree)
        };

        Self {
            true_palette,
            rbf: PhantomData::<F>,
            rbf_params: params.params,
            ref_palette,
            colorspace: params.colorspace,
        }
    }

    fn remap_pixel(&self, pixel: &mut image::Rgb<u8>) {
        let raw_color = &mut pixel.0;
        if self.true_palette.contains(raw_color) {
            return;
        }

        let colorf =
            self.colorspace
                .to_float(Color::new(raw_color[0], raw_color[1], raw_color[2], 255));

        let mut numerator = [0.0; 3];
        let mut denominator = 0.0;

        match &self.ref_palette {
            TreeOrVec::Vec(palette) => {
                for (i, &p_colorf) in palette.iter().enumerate() {
                    let delta = colorf - p_colorf;
                    let distance = delta.dot(&delta).sqrt();
                    let weight = F::radial_basis(self.rbf_params, distance);
                    let c = self.true_palette[i];

                    numerator[0] += c[0] as f64 * weight;
                    numerator[1] += c[1] as f64 * weight;
                    numerator[2] += c[2] as f64 * weight;
                    denominator += weight;
                }
            },
            TreeOrVec::Tree(nearest, tree) => {
                for Neighbour { item, distance } in tree.nearest_n(
                    &[colorf.r, colorf.g, colorf.b],
                    *nearest,
                    &kiddo::distance::squared_euclidean,
                ) {
                    // we have distance^2
                    let distance = distance.sqrt();
                    let weight = F::radial_basis(self.rbf_params, distance);
                    let color = self.true_palette[item as usize];

                    numerator[0] += color[0] as f64 * weight;
                    numerator[1] += color[1] as f64 * weight;
                    numerator[2] += color[2] as f64 * weight;
                    denominator += weight;
                }
            },
        }

        *raw_color = [
            (numerator[0] / denominator) as u8,
            (numerator[1] / denominator) as u8,
            (numerator[2] / denominator) as u8,
        ]
    }
}
