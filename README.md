<header>
    <h1 align="center">lutgen-rs</h1>
    <p align="center">
        <a href="https://crates.io/crates/lutgen"><img alt="crate" src="https://img.shields.io/crates/v/lutgen?style=for-the-badge" /></a>
        <a href="./LICENSE"><img alt="license" src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" /></a>
        <a href="https://github.com/ozwaldorf/lutgen-rs/actions/workflows/rust.yml"><img alt="ci" src="https://img.shields.io/github/actions/workflow/status/ozwaldorf/lutgen-rs/rust.yml?label=CI&style=for-the-badge" /></a>
        <a href="https://github.com/ozwaldorf/lutgen-rs/actions/workflows/publish.yml"><img alt="publish" src="https://img.shields.io/github/actions/workflow/status/ozwaldorf/lutgen-rs/publish.yml?label=Publish&style=for-the-badge" /></a>
    </p>
    <p align="center">
        A blazingly fast interpolated LUT generator using gaussian distribution for arbitrary and popular color palettes.
    </p>
</header>

---

## Example

<details>
    <summary>Catppuccin Mocha Hald Clut</summary>
    <img src="https://github.com/ozwaldorf/lutgen-rs/assets/8976745/d7eee751-5a3d-407f-9052-d16e28369635" />
</details>
<details>
    <summary>Example Image: Original and Corrected</summary>
    <img src="https://github.com/ozwaldorf/lutgen-rs/assets/8976745/76d5beaa-6ef8-4dec-8188-eeb56612df52" />
    <img src="https://github.com/ozwaldorf/lutgen-rs/assets/8976745/61a37d40-9423-419f-8199-5b24197e5485" />
</details>

## Usage

### CLI

Install

```bash
cargo install lutgen
```

Helptext

```text
A blazingly fast interpolated LUT generator using gaussian distribution for arbitrary and popular color palettes.

Usage: lutgen [OPTIONS] [CUSTOM_COLORS]... [COMMAND]

Commands:
  apply
          Correct an image using a hald clut, either generating it, or loading it externally
  help
          Print this message or the help of the given subcommand(s)

Arguments:
  [CUSTOM_COLORS]...
          Custom hexidecimal colors to add to the palette. If `-p` is not used to specify a base palette, at least 1 color is required

Options:
  -o, --output <OUTPUT>
          Path to write output to

  -p, --palette <PALETTE>
          Predefined popular color palettes. Use `lutgen -p` to view all options. Compatible with custom colors

  -l, --level <LEVEL>
          Hald level (ex: 8 = 512x512 image)
          
          [default: 8]

  -a, --algorithm <ALGORITHM>
          Algorithm to remap the LUT with
          
          [default: gaussian-rbf]

          Possible values:
          - shepards-method:
            Shepard's method (RBF interpolation using the inverse distance function). 
            Params: --power, --nearest
          - gaussian-rbf:
            Radial Basis Function interpolation using the Gaussian function. 
            Params: --euclide, --nearest
          - linear-rbf:
            Radial Basis Function interpolation using a linear function. Params: --nearest
          - gaussian-sampling:
            Optimized version of the original ImageMagick approach which applies gaussian noise to each color and averages nearest neighbors together. 
            Params: --mean, --std_dev, --iterations
          - nearest-neighbor:
            Simple, non-interpolated, nearest neighbor alorithm

  -m, --mean <MEAN>
          Gaussian sampling algorithm's mean parameter
          
          [default: 0]

  -s, --std-dev <STD_DEV>
          Gaussian sampling algorithm's standard deviation parameter
          
          [default: 20]

  -i, --iterations <ITERATIONS>
          Gaussian sampling algorithm's target number of samples to take for each color
          
          [default: 512]

      --power <POWER>
          Shepard algorithm's power parameter
          
          [default: 4]

      --euclide <EUCLIDE>
          Gaussian RBF's euclide parameter
          
          [default: 32]

      --nearest <NUM_NEAREST>
          Number of nearest palette colors to consider for RBF based algorithms. 0 uses unlimited (all) colors
          
          [default: 16]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

#### Examples

Generating a LUT

```bash
lutgen -p catppuccin-mocha -o mocha_lut.png
```

Correcting an image with a LUT generated on the fly

```bash
lutgen -p catppuccin-mocha apply assets/simon-berger-unsplash.jpg -o mocha_version.png
```

Correcting an image with a pre-generated LUT

```bash
lutgen apply --hald-clut mocha_lut.png input.jpg
```

Correcting Videos (using ffmpeg):

```bash
ffmpeg -i input.mkv -i hald_clut.png -filter_complex '[0][1] haldclut' output.mp4
```

### Library

> By default, the `bin` feature and dependencies are enabled.
> When used as a library, it's recommended to use `default-features = false` to minimalize the dependency tree and build time.

Generating a LUT (simple):

```rust
use exoquant::SimpleColorSpace;
use lutgen::{
    GenerateLut,
    interpolation::{
        GaussianRemapper, GaussianSamplingRemapper
    },
};

// Setup the palette to interpolate from
let palette = vec![
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
];

// Setup the fast Gaussian RBF algorithm
let (euclide, nearest) = (16.0, 0);
let remapper = GaussianRemapper::new(&palette, euclide, nearest, SimpleColorSpace::default());

// Generate and remap a HALD:8 for the provided palette
let hald_clut = remapper.generate_lut(8);
// hald_clut.save("output.png").unwrap();
    
// Setup the slower Gaussian Sampling algorithm
let (mean, std_dev, iters, seed) = (0.0, 20.0, 512, 420);
let remapper = GaussianSamplingRemapper::new(
    &palette, 
    mean, 
    std_dev, 
    iters, 
    seed, 
    SimpleColorSpace::default()
);

// Generate and remap a HALD:8 for the provided palette
let hald_clut = remapper.generate_lut(4);
// hald_clut.save("output.png").unwrap();
```

Applying a LUT:

```rust,ignore
use lutgen::identity::{generate, correct_image};

let identity = lutgen::identity::generate(8);
let mut image = image::open("example-image.png").unwrap().to_rgb8();

correct_image(&mut image, &identity);

// image.save("output.png").unwrap()
```

Remapping an image directly (advanced):

> Note: While the remappers can be used directly on an image, it's much faster to remap a LUT and correct an image with that.

```rust
use exoquant::SimpleColorSpace;
use lutgen::{
    GenerateLut,
    interpolation::{GaussianRemapper, InterpolatedRemapper},
};

// Setup the palette to interpolate from
let palette = vec![
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
];

// Setup a remapper
let (euclide, nearest) = (16.0, 0);
let remapper = GaussianRemapper::new(&palette, euclide, nearest, SimpleColorSpace::default());

// Generate an image (generally an identity lut to use on other images)
let mut identity = lutgen::identity::generate(8);

// Remap the image
remapper.remap_image(&mut identity);
// identity.save("v1_hald_8.png").unwrap();
```

## Tasks

- [x] Basic hald-clut identity generation
- [x] Gaussian Sampling interpolation for generating LUTs (thanks Gengeh for the original imagemagick strategy!)
- [x] Support a bunch of popular base color palettes (thanks Wezterm!)
- [x] Basic applying a lut to an image
- [x] Radial basis function interpolation for generating LUTs
- [ ] Interpolation for more accuracy when correcting with low level luts (<16)
- [ ] Replace exoquant and kiddo with a unified implementation of a k-d tree

## Sources 

- Hald Cluts: https://www.quelsolaar.com/technology/clut.html
- Sparse Hald Cluts: https://im.snibgo.com/sphaldcl.htm 
- RBF Interpolation: https://en.wikipedia.org/wiki/Radial_basis_function_interpolation
- Shepard's method: https://en.wikipedia.org/wiki/Inverse_distance_weighting
