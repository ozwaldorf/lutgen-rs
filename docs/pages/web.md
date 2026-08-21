---
layout: page
title: Web
permalink: web
---

# Lutgen Web

For those looking for an interactive experience, Lutgen Web is available at
[lut.sh/app](https://lut.sh/app). It runs the entire pipeline on the GPU via
WebGL2, so previews update as you drag the controls.

## Installing

On desktop and mobile devices, the webapp can be installed directly by browsers.

For example, in google chrome on android, tap the triple dot menu, then "Add to
homescreen", and finally "Install".

## Help

### Opening images

Click `Open image` in the header, or drag an image anywhere onto the preview.

### Saving edited output

`Save image` downloads the corrected image, named after the source file and the
palette. `HALD CLUT` exports the generated lookup table as a PNG, which can be
reused with the CLI's `--hald-clut` flag or any editor that supports HALD CLUTs.

### Comparing against the original

Drag anywhere on the preview to move the split between the original and the
corrected output.

### Choosing a palette

Every palette shipped with lutgen is available in the picker, searchable by
name, with a preview of its colors. Recently used palettes are listed first,
and `shuffle` jumps to a random one.

### Editing the palette

Use `edit` to enter custom colors as a list of hex values. The equivalent CLI
command is shown at the bottom of the sidebar and can be copied with one click.
