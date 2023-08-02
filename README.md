<header>
    <br>
    <div align="center">
        <img width="30%" src="https://github.com/ozwaldorf/lutgen-rs/assets/8976745/4163e179-ee73-4b24-8ad8-6c373e1d8711" />
    </div>
    <h2 align="center">lutgen-rs</h2>
    <p align="center">
        <a href="https://github.com/ozwaldorf/lutgen-rs/releases/latest"><img alt="crate" src="https://img.shields.io/github/v/release/ozwaldorf/lutgen-rs?style=for-the-badge" /></a>
        <a href="./LICENSE.md"><img alt="license" src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" /></a>
        <a href="https://github.com/ozwaldorf/lutgen-rs/actions/workflows/rust.yml"><img alt="ci" src="https://img.shields.io/github/actions/workflow/status/ozwaldorf/lutgen-rs/rust.yml?label=CI&style=for-the-badge" /></a>
        <a href="https://github.com/ozwaldorf/lutgen-rs/actions/workflows/publish.yml"><img alt="publish" src="https://img.shields.io/github/actions/workflow/status/ozwaldorf/lutgen-rs/publish.yml?label=Publish&style=for-the-badge" /></a>
    </p>
    <p align="center">
        A blazingly fast interpolated <a href="https://en.wikipedia.org/wiki/3D_lookup_table">LUT</a> generator and applicator for arbitrary and popular color palettes. Theme any image to your desktop colorscheme!
    </p>
</header>

---

## Example Output

### Hald Cluts

<details>
    <summary>Catppuccin Mocha</summary>
    <img src="docs/catppuccin-mocha-hald-clut.png" />
</details>
<details>
    <summary>Gruvbox Dark</summary>
    <img src="docs/gruvbox-dark-hald-clut.png" />
</details>
<details>
    <summary>Nord</summary>
    <img src="docs/nord-hald-clut.png" />
</details>

### Color Corrections

<details>
    <summary>Original Image</summary>
    <img src="docs/example-image.jpg" />
</details>
<details>
    <summary>Catppuccin Mocha</summary>
    <img src="docs/catppuccin-mocha.jpg" />
</details>
<details>
    <summary>Gruvbox Dark</summary>
    <img src="docs/gruvbox-dark.jpg" />
</details>
<details>
    <summary>Nord</summary>
    <img src="docs/nord.png" />
</details>

## Usage

> Note: The binary and library usages are fairly stable, but any release that does make any breaking changes as such, are bumped to 0.X.0

### CLI

#### Source

```bash
git clone https://github.com/ozwaldorf/lutgen-rs
cd lutgen-rs
cargo install --path .
```

#### Releases

| Packaging Status | Installation Command |
|------------------|----------------------|
| [![Crates.io](https://repology.org/badge/version-for-repo/crates_io/rust:lutgen.svg?header=crates.io)](https://crates.io/crates/lutgen) | `cargo install lutgen` |
| [![AUR](https://repology.org/badge/version-for-repo/aur/lutgen.svg?header=AUR%20(bin))](https://aur.archlinux.org/packages/lutgen-bin) | `yay -S lutgen-bin` |
| [![AUR](https://img.shields.io/badge/any_text-rolling-lightgrey?label=AUR%20(git))](https://aur.archlinux.org/packages/lutgen-git) | `yay -S lutgen-git` |
| [![Alpine](https://repology.org/badge/version-for-repo/alpine_edge/lutgen.svg?header=Alpine%20Linux%20\(testing\))](https://pkgs.alpinelinux.org/package/edge/testing/x86_64/lutgen) | `apk add lutgen` |

#### Helptext

```text
Usage: lutgen <COMMAND>

Commands:
  generate     Generate a hald clut for external or manual usage
  apply        Correct an image using a hald clut, either generating it, or loading it externally
  completions  Generate shell completions
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

#### Examples

Correcting an image

```bash
# Builtin palette
lutgen apply -p catppuccin-mocha docs/example-image.jpg -o mocha_version.jpg

# Custom colors 
lutgen apply docs/example-image.jpg -- "#ABCDEF" ffffff 000000

# Custom palette file 
lutgen apply docs/example-image.jpg -- $(cat palette.txt)

# Multiple images
lutgen apply image1.png image2.png *.jpg -p catppuccin-mocha

# Using an external LUT 
lutgen apply --hald-clut mocha_lut.png docs/example-image.jpg
```

Generating a standalone LUT for external or manual usage

```bash
# Builtin palette
lutgen generate -p catppuccin-mocha -o mocha_lut.png

# Custom colors
lutgen generate -o custom.png -- "#ABCDEF" ffffff 000000

# Custom palette file with hex codes
lutgen generate -o custom.png -- $(cat palette.txt)
```

Correcting videos (using ffmpeg):

```bash
ffmpeg -i input.mkv -i hald_clut.png -filter_complex '[0][1] haldclut' output.mp4
```

Zsh Completions

```bash
lutgen completions zsh > _lutgen
sudo mv _lutgen /usr/local/share/zsh/site-functions/
```

### Library

> By default, the `bin` feature and dependencies are enabled.
> When used as a library, it's recommended to use `default-features = false` to minimalize the dependency tree and build time.

Generating a LUT (simple):

```rust
use lutgen::{
    GenerateLut,
    interpolation::{
        GaussianRemapper, GaussianSamplingRemapper
    },
};
use lutgen_palettes::Palette;

// Get a premade palette
let palette = Palette::CatppuccinMocha.get();

// Setup the fast Gaussian RBF algorithm
let (shape, nearest, lum_factor) = (96.0, 0, 1.0);
let remapper = GaussianRemapper::new(&palette, shape, nearest, lum_factor);

// Generate and remap a HALD:8 for the provided palette
let hald_clut = remapper.generate_lut(8);

// hald_clut.save("output.png").unwrap();

// Setup another palette to interpolate from, with custom colors
let palette = vec![
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
];

// Setup the slower Gaussian Sampling algorithm
let (mean, std_dev, iters, lum_factor, seed) = (0.0, 20.0, 512, 1.0, 420);
let remapper = GaussianSamplingRemapper::new(
    &palette,
    mean,
    std_dev,
    iters,
    lum_factor,
    seed
);

// Generate and remap a HALD:4 for the provided palette
let hald_clut = remapper.generate_lut(4);

// hald_clut.save("output.png").unwrap();
```

Applying a LUT:

```rust
use image::open;

use lutgen::{
    identity::correct_image,
    interpolation::GaussianRemapper,
    GenerateLut,
};
use lutgen_palettes::Palette;

// Generate a hald clut
let palette = Palette::CatppuccinMocha.get();
let remapper = GaussianRemapper::new(&palette, 96.0, 0, 1.0);
let hald_clut = remapper.generate_lut(8);

// Save the LUT for later
hald_clut.save("docs/catppuccin-mocha-hald-clut.png").unwrap();

// Open an image to correct
let mut external_image = open("docs/example-image.jpg").unwrap().to_rgb8();

// Correct the image using the hald clut we generated
correct_image(&mut external_image, &hald_clut);

// Save the edited image
external_image.save("docs/catppuccin-mocha.jpg").unwrap()
```

Remapping an image directly (advanced):

> Note: While the remappers can be used directly on an image, it's much faster to remap a LUT and correct an image with that.

```rust
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
let (shape, nearest, lum_factor) = (96.0, 0, 1.0);
let remapper = GaussianRemapper::new(&palette, shape, nearest, lum_factor);

// Generate an image (generally an identity lut to use on other images)
let mut hald_clut = lutgen::identity::generate(8);

// Remap the image
remapper.remap_image(&mut hald_clut);

// hald_clut.save("output.png").unwrap();
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
- Editing with Hald Cluts: https://im.snibgo.com/edithald.htm
- Sparse Hald Cluts: https://im.snibgo.com/sphaldcl.htm 
- RBF Interpolation: https://en.wikipedia.org/wiki/Radial_basis_function_interpolation
- Shepard's method: https://en.wikipedia.org/wiki/Inverse_distance_weighting
- Oklab Colorspace: https://bottosson.github.io/posts/oklab/

## Special Thanks

- [Stonks3141](https://github.com/Stonks3141) for maintaining the Alpine Linux package

## Star History

<a href="https://star-history.com/#ozwaldorf/lutgen-rs&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ozwaldorf/lutgen-rs&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ozwaldorf/lutgen-rs&type=Date" />
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ozwaldorf/lutgen-rs&type=Date" />
  </picture>
</a>
