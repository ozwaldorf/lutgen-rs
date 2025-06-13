---
layout: page
title: Examples
permalink: /examples
---

# Examples and use-cases

## Basic usages

Edit `my-image` into the catppuccin colorscheme

```bash
lutgen apply --palette catpuccin-mocha my-image.png
```

If results could be better, try tweaking some parameters. For example:

```bash
lutgen apply --preserve catpuccin-mocha --lum 0.5 --preserve my-image.png
# or for short
lutgen a -p catppuccin-mocha -PL0.5 my-image.png
```

## Generating raw LUTs

```bash
# Builtin palette
lutgen generate -p catppuccin-mocha

# Custom colors
lutgen generate -o custom.png -- "#ABCDEF" ffffff 000000
```

## Color palettes

```bash
# Preview all palettes (there's a lot)
lutgen palette all

# Finding a palette name with grep
lutgen palette names | grep 'gruvbox'

# View a palette's colors
lutgen palette catppuccin-mocha
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

```bash
# create a patch file
lutgen patch ./**/*.css --palette catppuccin > catppuccin.patch

# apply changes directly, with no patchfile output
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

