---
layout: page
title: Tips and tricks
permalink: tips
---

# Tips and tricks

## Palette optimization

Lutgen produces the best results with palettes that have 2 things:

1. A selection of primary colors, ideally with a dark and light variant
2. A few monochromatic variants, generally the background/foreground colors

Most color palettes fit this, providing a dark and light color and a bunch of background and foreground colors. For example, Catppuccin has 14 main colors and 12 monochrome colors, which produces exceptional results.

## Gaussian RBF

The shape parameter in the default algorithm effectively creates more or less blending between colors in the palette, where bigger numbers equal less blending. Results are heavily dependant on the number of nearest colors used.

To make colors closer to the originals, with less blending:

    lutgen apply --shape 256

To make colors more blended, and smoother:

    lutgen apply --shape 64

## Luminocity factor

If the image results are too monochromatic and don't have enough color, or inversely, too colorful and not enough monochrome, the LUT can be shifted to prefer either direction using the luminocity factor flag.

To shift the LUT to be more colorful, use luminocity factors below 1.0:

    lutgen apply --lum 0.5 my-image.jpg

To shift the LUT to have less colors, use luminocity factors above 1.0:

    lutgen apply --lum 1.5 my-image.jpg

## Preserve

Usually used in combination with the luminocity factor, the `--preserve` flag can be used to ensure the output colors preserve the original images luminocity. In laymans terms, this effectively preserves the image's contrast, and generally improves gradients.

For example:

    lutgen apply --preserve my-image.jpg
    lutgen apply --preserve --lum 0.5 my-image.jpg
