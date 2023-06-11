use std::f64;

use exoquant::{Color, ColorSpace, Colorf};

use super::InterpolatedRemapper;

pub struct ShepardRemapper<'a, CS: ColorSpace + Sync> {
    true_palette: &'a [[u8; 3]],
    palette: Vec<Colorf>,
    colorspace: CS,
    power: f64,
    nearest: usize,
}

pub struct ShepardsV1Params<CS: ColorSpace> {
    pub power: f64,
    pub nearest: usize,
    pub colorspace: CS,
}

fn partition(array: &mut [(usize, f64)], mut low: usize, high: usize) -> usize {
    let pivot = array[high];
    for j in low..high {
        if array[j].1 <= pivot.1 {
            array.swap(low, j);
            low += 1;
        }
    }
    array.swap(low, high);
    low
}

fn quickselect(array: &mut [(usize, f64)], low: usize, high: usize, k: usize) {
    if low < high {
        let pivot = partition(array, low, high);
        if k < pivot {
            quickselect(array, low, pivot - 1, k);
        } else if k > pivot {
            quickselect(array, pivot + 1, high, k);
        }
    }
}

impl<'a, CS: ColorSpace + Sync> InterpolatedRemapper<'a> for ShepardRemapper<'a, CS> {
    type Params = ShepardsV1Params<CS>;

    fn new(palette: &'a [[u8; 3]], params: Self::Params) -> Self {
        let true_palette = palette;
        let palette = true_palette
            .iter()
            .map(|c| {
                params
                    .colorspace
                    .to_float(Color::new(c[0], c[1], c[2], 255))
            })
            .collect();

        Self {
            true_palette,
            palette,
            power: params.power,
            nearest: params.nearest,
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

        let mut distances: Vec<_> = self
            .palette
            .iter()
            .enumerate()
            .map(|(i, &p_colorf)| {
                // Compute the distance from the input to the current palette color
                let delta = colorf - p_colorf;
                let distance = delta.dot(&delta).sqrt();
                (i, distance)
            })
            .collect();

        let len = distances.len();
        if self.nearest > 0 && self.nearest != len {
            quickselect(&mut distances, 0, len - 1, self.nearest - 1);
            distances.truncate(self.nearest);
        }

        let mut numerator = [0.0; 3];
        let mut denominator = 0.0;
        for (i, distance) in distances {
            // Compute the weight for the distance using the
            // inverse distance weighting (IDW) formula.
            let weight = 1.0 / distance.powf(self.power);

            // Incrementally compute the weighted result
            let color = self.true_palette[i];
            numerator[0] += color[0] as f64 * weight;
            numerator[1] += color[1] as f64 * weight;
            numerator[2] += color[2] as f64 * weight;
            denominator += weight;
        }

        *raw_color = [
            (numerator[0] / denominator) as u8,
            (numerator[1] / denominator) as u8,
            (numerator[2] / denominator) as u8,
        ]
    }
}
