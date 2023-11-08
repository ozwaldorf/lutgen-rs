use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use lutgen::{
    identity::correct_image,
    interpolation::{GaussianRemapper, LinearRemapper, ShepardRemapper},
    GenerateLut, Image,
};
use lutgen_palettes::Palette;

fn benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("Gaussian RBF");
    g.sample_size(50);
    for i in 4..=16u8 {
        g.bench_with_input(BenchmarkId::new("Level", i), &i, |b, i| {
            b.iter(|| gaussian_rbf(*i));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("Shepards Method");
    g.sample_size(50);
    for i in 4..=16u8 {
        g.bench_with_input(BenchmarkId::new("Level", i), &i, |b, i| {
            b.iter(|| shepards_method(*i));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("Generate Identity");
    g.sample_size(100);
    for i in 4..=16u8 {
        g.bench_with_input(BenchmarkId::new("Level", i), &i, |b, i| {
            b.iter(|| generate(*i));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("Apply");
    g.sample_size(100);
    for i in 4..=16u8 {
        g.bench_with_input(BenchmarkId::new("HALD", i), &i, |b, i| {
            let a = GaussianRemapper::new(Palette::Carburetor.get(), 96.0, 16, 1.0, true);
            let lut = a.generate_lut(*i);
            let image = image::open("docs/example-image.jpg")
                .expect("failed to load image")
                .to_rgb8();
            b.iter(|| apply(&lut, image.clone()));
        });
    }
}

fn generate(level: u8) {
    let identity = lutgen::identity::generate(level);
    black_box(identity);
}

fn gaussian_rbf(level: u8) {
    let algorithm = GaussianRemapper::new(Palette::Carburetor.get(), 96.0, 16, 1.0, true);
    let lut = algorithm.generate_lut(level);
    black_box(lut);
}

fn shepards_method(level: u8) {
    let algorithm = ShepardRemapper::new(Palette::Carburetor.get(), 16.0, 16, 1.0, true);
    let lut = algorithm.generate_lut(level);
    black_box(lut);
}

fn apply(lut: &Image, mut img: Image) {
    correct_image(&mut img, lut);
    black_box(img);
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
