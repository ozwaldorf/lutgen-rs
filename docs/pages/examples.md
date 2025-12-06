---
layout: page
title: Examples
permalink: /examples
---

# Examples and use-cases

## Basic usages

Edit `my-image` into the catppuccin colorscheme

```bash
lutgen apply --palette catppuccin-mocha my-image.png
```

If results could be better, try tweaking some parameters. For example:

```bash
lutgen apply --preserve catppuccin-mocha --lum 0.5 --preserve my-image.png
# or for short
lutgen a -p catppuccin-mocha -PL0.5 my-image.png
```

## Generating and extracting raw LUTs

Raw hald-cluts can be generated directly for use in other software or using in lutgen manually:

```bash
# Generate from a builtin palette
lutgen generate -p catppuccin-mocha

# Generate from custom colors
lutgen generate -o custom.png -- "#ABCDEF" ffffff 000000
```

Lutgen also supports extracting from existing image(s), useful especially for film emulation or replicating a look and feel:

```bash
# Extract a lut from an existing image
lutgen extract -o custom-hald-clut.png my-image.png

# Extract a lut using multiple source images
lutgen extract -o polaroid-669-hald-clut.png polaroid-669-samples/*.png
```

Raw hald-cluts can then be imported into other image editing software, or applied to an image directly using lutgen:

```bash
lutgen apply --hald-clut polaroid-669-clut.png another-image.png
```

## Color palettes

#### Preview all palettes (there's a lot)

```bash
lutgen palette all
```

#### View a palette's colors

```bash
lutgen palette catppuccin-mocha
```

#### Searching and previewing palettes with fzf

```bash
lutgen palette names | fzf --preview 'lutgen palette --ansi {}'
```

#### Finding a palette name with grep

```bash
lutgen palette names | grep 'gruvbox'
```

## Custom color palettes

```bash
# Copy a palette to the custom palette dir for modifying and overriding
lutgen palette carburetor > ~/.config/lutgen/carburetor

# Custom palette file with whitespace separated hex colors (linux example shown)
echo "fff 555 000 abcdef deadbe" > ~/.config/lutgen/my-palette-name
lutgen generate -p my-palette-name
```

## Patching text files

#### Creating a patch file

```bash
lutgen patch ./**/*.css --palette catppuccin > catppuccin.patch
```

#### Apply changes directly, with no patchfile output

```bash
lutgen patch -wn ./**/*.css -p catppuccin
```

## Setting up shell auto-completions

Lutgen supports completions for bash, zsh, and fish.
Most package managers will setup the completions when installing the package,
but can also be manually done.

#### ZSH

```bash
lutgen --bpaf-complete-style-zsh > _lutgen
sudo mv _lutgen /usr/local/share/zsh/site-functions/
```

