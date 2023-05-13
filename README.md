# lutgen-rs

A Library and CLI Tool to generate interpolated lookup tables (hald-clut) for arbitrary color palettes using guassian distribution.

## Usage

### CLI

```bash
cargo install lutgen --version 0.1.0-alpha.0
```

```text
$ lutgen -h

Usage: lutgen [OPTIONS]

Options:
  -a <ALGORITHM>                 Algorithm to generate the LUT with [default: v1] [possible values: v1, v0]
  -o, --output <OUTPUT>          Path to write the generated file to. Defaults to the current dir with some parameters (ex: `./hald_clut_v1_4_20_512.png`)
  -l, --level <LEVEL>            HaldCLUT color depth. 8 bit = 512x512 image [default: 8]
  -m, --mean <MEAN>              Mean for the gaussian distribution [default: 4]
  -s, --std-dev <STD_DEV>        Standard deviation for the gaussian distribution [default: 20]
  -i, --iterations <ITERATIONS>  Number of iterations to average together [default: 512]
  -h, --help                     Print help (see more with '--help')
  -V, --version                  Print version
```

### Library

> By default, the `bin` feature and dependencies are enabled.
> When used as a library, it's recommended to use `default-features = false` to minimalize the dependency tree and build time.

Simple usage:

```rust
use exoquant::Color;

// Setup the palette and parameters
let palette = vec![
    Color::new(255, 0, 0, 255),
    Color::new(0, 255, 0, 255),
    Color::new(0, 0, 255, 255),
];

// Generate the LUT using the v1 algorithm:
let lut = lutgen::generate_v1_lut(&palette, 8, 4.0, 20.0, 512, 0);
// Or, v0: lutgen::generate_v0_lut(&palette, 8, 4.0, 20.0, 512, 0);
```

Advanced usage:

```rust
use exoquant::Color;

// Generate the base identity
let identity = lutgen::identity::generate(8);

// Setup the palette and parameters
let palette = vec![
    Color::new(255, 0, 0, 255),
    Color::new(0, 255, 0, 255),
    Color::new(0, 0, 255, 255),
];
let mean = 4.0;
let std_dev = 20.0;
let iters = 512;
let seed = 0;

// Remap the identity using v1:
let output_v1 = lutgen::interpolated_remap::v1::remap_image(identity, &palette, mean, std_dev, iters, seed);
// Or, v0: lutgen::interpolated_remap::v0::remap_image(&identity, &palette, mean, std_dev, iters, seed);
```

