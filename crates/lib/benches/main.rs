use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use lutgen::identity::correct_image;
use lutgen::interpolation::{
    GaussianRemapper,
    GaussianSamplingRemapper,
    InterpolatedRemapper,
    ShepardRemapper,
};
use lutgen::{GenerateLut, RgbImage, RgbaImage};
use lutgen_palettes::Palette;

fn benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("generate_identity");
    g.sample_size(100);
    // hald 4, 8, 12, 16
    for i in (1..=4).map(|i| i * 4) {
        g.bench_with_input(BenchmarkId::new("hald", i), &i, |b, i| {
            b.iter(|| generate(*i));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("remap_gaussian_rbf");
    g.sample_size(25);
    for i in (1..=4).map(|i| i * 4) {
        g.bench_with_input(BenchmarkId::new("hald", i), &i, |b, i| {
            b.iter(|| hald(*i, gaussian_rbf));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("remap_shepards_method");
    g.sample_size(25);
    for i in (1..=4).map(|i| i * 4) {
        g.bench_with_input(BenchmarkId::new("hald", i), &i, |b, i| {
            b.iter(|| hald(*i, shepards_method));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("remap_gaussian_sampling");
    g.sample_size(25);
    for i in (1..=4).map(|i| i * 4) {
        g.bench_with_input(BenchmarkId::new("hald", i), &i, |b, i| {
            b.iter(|| hald(*i, gaussian_sampling));
        });
    }
    drop(g);

    let mut g = c.benchmark_group("remap_gaussian_sampling");
    g.sample_size(100);
    g.bench_function(BenchmarkId::new("pixel", 1), |b| {
        b.iter(|| {
            let mut pixel = [63, 127, 255, 255].into();
            gaussian_sampling().remap_pixel(&mut pixel);
            black_box(pixel);
        })
    });

    drop(g);

    let mut g = c.benchmark_group("apply");
    g.sample_size(100);
    for i in (1..=4).map(|i| i * 4) {
        g.bench_with_input(BenchmarkId::new("hald", i), &i, |b, i| {
            let lut = gaussian_rbf().generate_lut(*i);
            let image = image::open("../../docs/assets/example-image.jpg")
                .expect("failed to load image")
                .to_rgba8();

            b.iter(|| apply(&lut, image.clone()));
        });
    }
    drop(g);
}

fn generate(level: u8) {
    let identity = lutgen::identity::generate(level);
    black_box(identity);
}

fn hald<'a, F: FnOnce() -> T, T: InterpolatedRemapper<'a> + GenerateLut<'a>>(level: u8, fun: F) {
    let lut = GenerateLut::generate_lut(&fun(), level);
    black_box(lut);
}

fn gaussian_rbf() -> GaussianRemapper {
    GaussianRemapper::new(Palette::Carburetor.get(), 96.0, 16, 1.0, true)
}

fn shepards_method() -> ShepardRemapper {
    ShepardRemapper::new(Palette::Carburetor.get(), 16.0, 16, 1.0, true)
}

fn gaussian_sampling() -> GaussianSamplingRemapper<'static> {
    GaussianSamplingRemapper::new(
        Palette::Carburetor.get(),
        0.0,
        20.0,
        512,
        1.0,
        42080085,
        false,
    )
}

fn apply(lut: &RgbImage, mut img: RgbaImage) {
    correct_image(&mut img, lut);
    black_box(img);
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
