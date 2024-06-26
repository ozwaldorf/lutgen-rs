<header>
    <br>
    <div align="center">
        <img width="30%" src="https://github.com/ozwaldorf/lutgen-rs/assets/8976745/4163e179-ee73-4b24-8ad8-6c373e1d8711" />
    </div>
    <h2 align="center">lutgen-rs</h2>
    <p align="center">
        <a href="https://github.com/ozwaldorf/lutgen-rs/releases/latest"><img alt="crate" src="https://img.shields.io/github/v/release/ozwaldorf/lutgen-rs?style=for-the-badge" /></a>
        <a href="./LICENSE.md"><img alt="license" src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" /></a>
        <a href="https://github.com/ozwaldorf/lutgen-rs/actions/workflows/publish.yml"><img alt="publish" src="https://img.shields.io/github/actions/workflow/status/ozwaldorf/lutgen-rs/publish.yml?label=Publish&style=for-the-badge" /></a>
        <a href="https://garnix.io"><img alt="ci" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fgarnix.io%2Fapi%2Fbadges%2Fozwaldorf%2Flutgen-rs&style=for-the-badge&logo=%20&label=garnix&labelColor=grey" /></a>
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

> Note: The binary usages are fairly stable, but any release that does make any breaking changes as such, are bumped to 0.X.0

### CLI

[![Packaging status](https://repology.org/badge/vertical-allrepos/lutgen.svg)](https://repology.org/project/lutgen/versions)

#### Source

```bash
git clone https://github.com/ozwaldorf/lutgen-rs
cd lutgen-rs
cargo install --path .
```

#### Nix flake

A nix flake is available and can be run easily with:

```bash
nix run github:ozwaldorf/lutgen-rs
```

Cache is provided via https://garnix.io

#### Helptext

```text
Usage: lutgen <COMMAND>

Commands:
  generate     Generate a hald clut for external or manual usage
  apply        Correct an image using a hald clut, either generating it, or loading it externally
  patch        Generate a patch for rgb colors inside text files
  palette      Print palette colors and names
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

Palletes

```bash
# Preview all palettes
lutgen palette

# Copy a palette to a file for tweaking
lutgen palette carburetor > carburetor.txt

# Finding a palette name with grep
lutgen palette --name-only | grep 'gruvbox'
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

See the latest documentation on [docs.rs](https://docs.rs/lutgen)

## Planned features

- [ ] Interpolation for more accuracy when correcting with low level luts (<16)
- [ ] Hardware acceleration for applying luts to images

## Sources

- Hald Cluts: https://www.quelsolaar.com/technology/clut.html
- Editing with Hald Cluts: https://im.snibgo.com/edithald.htm
- Sparse Hald Cluts: https://im.snibgo.com/sphaldcl.htm
- RBF Interpolation: https://en.wikipedia.org/wiki/Radial_basis_function_interpolation
- Shepard's method: https://en.wikipedia.org/wiki/Inverse_distance_weighting
- Oklab Colorspace: https://bottosson.github.io/posts/oklab/

## Special Thanks

- [Stonks3141](https://github.com/Stonks3141) for maintaining the Alpine Linux package
- All the nixpkgs maintainers
