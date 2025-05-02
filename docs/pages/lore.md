---
layout: page
title: Lore
permalink: lore
---

# Lore

## What is lutgen used for?

Lutgen is primarily tailored for desktop theming, used to color grade wallpapers to match with color palettes used in desktop and application themes. However, it can also be used as a generic LUT generation tool for graphic design and used inside popular image editors (that support Hald Cluts).

## How does it work?

Lutgen works by creating a Hald-Clut lookup-table, and applying it to a given image (color grading). It can also generate just the lookup tables for use in external programs, as well as a patch mode that works on text files containing css styled colors.

## Inspiration

The original algorithm to generate the LUTs was implemented by Gingeh, orignally using a shader, but switched to an approach using imagemagick.

The basic algorithm is as such:

1. Create X number of copies (iterations) of the base identity LUT
2. Apply noise to each interation (with some parameters)
3. Apply the palette's nearest neighbors to each iteration
3. Average all iterations together for the final output LUT

This approach was effective, but slow, taking around 3 minutes to generate a full scale HALD:16.

## Initial Implementation

Lutgen's first and original algorithm, `Gaussian Sampling`, is an optimization of this approach reducing the time to generate by several orders of magnitude, to about 3 seconds.

The optimized algorithm is as such:

1. In parallel, for each pixel in the identity LUT:
    1. Store a variable for incrementally computing the averaged output
    2. Create a variant of the pixel by applying gaussian noise (with parameters)
    3. Find the nearest neighbor from the palette to the variant
    4. Divide the nearest neighbor's channels by the total number of variants (iterations)
    5. Add the weighted variant to the average variable
    6. Repeat for the total number of variants
2. Collect all modified pixels into the final output LUT

This produces nearly identical results to the original method, while exposing more parameters to tweak for the guassian noise.

## Oklab

I was suggested to look into Oklab as a better alternative to doing math on colors with RGB. This worked out perfectly, as finding nearest neighbors and averaging colors in Oklab is much more perceptually accurate, and results are smooth and look great!

## Optimization

After the first algorithm was implemented, I sought out to find an even faster algorithm, knowing that there had to be another way. I had found an existing method of producing sparce hald-cluts, using Shepard's Method (aka inverse distance weighting) outlined [here](https://im.snibgo.com/sphaldcl.htm#procmod), so I sought to write my own implementation of this.

This proved successful, further reducing the time from several seconds using gaussian sampling, to a few hundred milliseconds using Shepard's Method (IDW). However, the output from Shepard's method was a little different from guassian sampling, and I knew it should be possible to have a similar algorithm that produces results as good looking as gaussian sampling.

I was researching RBF interpolation (Radial-Basis-Function), and realized that Shepard's Method is just RBF interpolation where inverse distance is the radial basis. There are several alternative functions to interpolate the LUT, notably a linear function, and the gaussian function.

In the end, Guassian RBF is the fastest, looks exceptionally similar to the original algorithm, and therefore is now the default algorithm in lutgen!

## Fine tuning

Now that we have the speed and quality, the next thing was allowing users to additionally fine tune the LUT other than the algorithm's specific parameters. Luminosity factor was added to support this, which allows shifting the LUT to prefer more color or brightness when matching. Shortly after, the `--preserve` flag was added to retain the original image's brightness and contrast.
