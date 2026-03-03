# Changelog

All notable changes to this project will be documented in this file.

---

## [Unreleased]

### Lutgen CLI

New default algorithm: Gaussian Blur (`--gaussian-blur` / `-B`) replaces Gaussian RBF as the default. This algorithm builds a nearest-neighbor LUT then applies separable Gaussian blur on OKLab color values, producing results mathematically equivalent to gaussian sampling but significantly faster.

Help text improvements:
- Arguments are now grouped by algorithm for clarity
- Custom usage lines per algorithm avoid confusing flag combinations
- Alternative algorithm flags now appear in shell completions

### lutgen (library)

- New `GaussianBlurRemapper` algorithm with optimized separable blur implementation
- Parallelized OKLab-to-RGB conversion in LUT generation
- Reduced heap allocations in parallel nearest-neighbor tree building

### lutgen-palettes

- Added Evergarden palettes (fall, spring, summer, winter) (#41)

### Lutgen Studio

#### Web app

Lutgen Studio now runs in the browser as a Progressive Web App (PWA), available at [lut.sh/app](https://lut.sh/app). The web version includes a service worker for offline support, image processing in a background worker thread, and a responsive layout that adapts to mobile and narrow windows.

Other improvements:
- Export submenu with format selection on web
- Icons in menus and docs button in top bar
- Tooltips for remapper settings
- Zoom percentage display in statusline
- Visual feedback with dimmed image and spinner during processing
- Fallback to re-encode without transparency if encoding fails

---

## [Studio v0.3.0] - 2025-08-01

- Moved preserve option to common args for all algorithms
- Added link to topbar when new updates are available
- Update lib to 0.14 and use newly migrated par methods
- Skip detecting hald clut levels whenever possible

---

## [CLI v1.0.1] - 2025-08-01

- Moved `preserve` to common args for all algorithms
- Updated yanked kiddo version and all other dependencies
- Use migrated par methods
- Skip detecting hald clut levels whenever possible

### lutgen v0.14.0

- Added `preserve` argument to gaussian sampling and nearest neighbor algorithms
- Migrate all existing methods that use rayon into `par_*` variants, gated by the rayon feature flag, leaving the previous undecorated methods as single threaded
- Added `correct_image_with_level` to skip detecting when level is already known
- Reduced image dependency feature-set to only what's needed

---

## [Studio v0.2.1] - 2025-07-16

- Hotfix: Store output for saving in save_as, handle errors gracefully

---

## [Studio v0.2.0] - 2025-07-15

### Lutgen Studio

- Native cross-platform file picker
- Use current image as prefilled save as path
- Use previous directory for picking file to open
- Display original over image when toggled
- Enable panning and zooming image
- Use nearest neighbor image rendering for pixel perfect representation
- Command arguments (image path to open, verbosity flags)
- Argument reset button
- Button to copy equivalent cli arguments for lutgen cli
- New filter-able palette selection box (replaces basic dropdown)
- Full support for custom palettes (load from selection box, save button in palette editor)
- Dynamically resize palette colors to fill up sidebar
- Styling consistency tweaks

### lutgen v0.13.2

- Updated yanked kiddo version and all other dependencies

### lutgen-palettes v0.4.1

- Added `Into<&'static str>` derive with strum

---

## [Studio v0.1.1] - 2025-06-27

### Lutgen Studio

- Sidebar is now resizable and widgets fill the space
- Image preview is now clickable and toggles between the original and edited image
- About/help dialog (opens on first start and through file menu)
- Ongoing apply tasks abort when re-requesting with new settings
- Many many internal refactors

### lutgen v0.13.1

- Added new method variants `*_with_interrupt` to allow tasks to be abortable

---

## [CLI v1.0.0] - 2025-06-25

Lutgen CLI is officially moving to a stable 1.0 version under a new separate crate `lutgen-cli`.

This means that the current CLI usage is stable and will not drastically change unless another major version bump is made. There is no difference between lutgen 0.12.1 and the new 1.0.0 version.

### Library Separation

The rust library will bump to a clean version as 0.13.0 and will track separately from the CLI going forward.

---

## [Studio v0.1.0] - 2025-06-25

Initial release for the new GUI, Lutgen Studio!

---

## [CLI v0.12.1] - 2025-06-20

- Added `extract` subcommand, extracting a LUT from input image(s) to apply to other images directly, for example for film emulation
- Added `lutgen palette --ansi ...` flag to force printing ansi sequences in palette previews (enables using fzf to search and preview palettes)

---

## [CLI v0.12.0] - 2025-04-16

### CLI

- Adds support for transparent images
- Adds support for animated gifs

### Library

- `lutgen::Image` type is changed to RGBA8 to support transparency
- A new type is added `ClutImage` for hald cluts specifically, which is still RGB8

---

## [CLI v0.11.2] - 2024-10-10

- Fallback to usage text for each subcommand

---

## [CLI v0.11.1] - 2024-10-10

- Fixed fish completions past the first argument. Not as fancy, but actually works.
- Fixed formatting for available image extensions in helptext
- Bump dependencies

---

## [CLI v0.11.0] - 2024-09-06

### Builtin Palettes

If possible, please verify any palettes you use!

- Palettes are now scraped from a few new sources with (theoretically) more accuracy. All palettes are deduplicated with `-base16` `-terminal-sexy` and `-gogh` suffixes removed.
- Added Swamp & Swamp-Light by @masroof-maindak in #23

### CLI

#### Custom Palette Support

Custom palettes can be added to `$LUTGEN_DIR` or `<CONFIG DIR>/lutgen`, for example:

- Linux: `/home/alice/.config/lutgen`
- macOS: `/Users/Alice/Library/Application Support/lutgen`
- Windows: `C:\Users\Alice\AppData\Roaming\lutgen`

Names are case-insensitive and parsed from the file stem, minus any file extensions. For example, `~/.config/lutgen/My-palette.txt` would be available to use as `my-palette`.

Custom palettes work anywhere a builtin palette would, ie `lutgen apply -p my-palette`

#### Migrated from clap to bpaf

- Explicit separation between different algorithms adjacent arguments (ie, `--gaussian-sampling --mean 0 --std-dev 20 --iters 128`)
- Flag aliases everywhere, due to context aware parsing of algorithm args preventing conflicts
- Subcommand aliases (ie `apply/a`, `generate/g`, `patch/p`)
- Improved dynamic completions for files, palettes, and args
- Improved helptexts and added man page (`man 1 lutgen`)
- (maintainers): new completion generation commands: `--bpaf-complete-style-<bash/zsh/fish>`

#### Other Changes

- Single file default output behavior restored (ie `myimage-gruvbox.png`). This now has an optional flag `-d` which can restore enabling directory output mode for single files (ie `gruvbox/myimage.png`)
- Improved lut caching eliminates false hits and ensures cache paths are unique to the individual colors (ie luts cached via `lutgen apply -c ...`)
- Palettes can be extended with custom colors by using them simultaneously
- Guess image types

---

## [CLI v0.10.1] - 2024-05-14

### Library

- fix: skip over creation of the tree on small palette by @qti3e in #18

### CLI

- feat: patch subcommand for text files, supporting `#ABCDEF`, `rgb(4, 2, 0)`, and `rgba(0, 1, 2, 0.5)`
- feat: support 3 color hex codes in cli args (ie, `#000`)
- refactor: all spinners now print in stderr (to allow patch command's output to be easily piped from stdout)
- fix: output path logic

---

## [CLI v0.10.0] - 2024-03-15

### CLI (breaking change)

- All default outputs for single files will now match the behavior of multi file operations. The output path will be stored in a directory named by the palette, unless otherwise specified.
- The default shape parameter for GaussianRBF, the default interpolation algorithm, is now 128

### Library

- Simple benchmarks
- Performance: refactor for kiddo v4
- Performance: optimize identity::generate
- Performance: enable fat lto, allow loop vectorization during build

### Palettes

- Updated Biscuit Light by @ronindoll in #10
- Added swamp palette by @masroof-maindak in #11
- Added Dark Decay colorscheme by @Alxhr0 in #13
- Added Decayce by @Alxhr0 in #15

---

## [CLI v0.9.0] - 2023-10-12

### Library

- Internal refactors, moved typed rbf algorithms into a macro implementation (yay meta programming!).

### CLI

Added `--preserve` flag for all rbf algorithms, which will preserve the original images luminosity values, after interpolation.
This allows for combining `--lum 0 --preserve` to ignore all luminosity when interpolating, and to use the original images
luminosity instead, for some really nice results.

### Palettes

- Fixed Oxocarbon colors
- Added Carburetor palettes (regular, cool, and warm)
- Added Biscuit Dark & Biscuit Light palettes (#8)
- Added Mountain Fuji palette (#9)

---

## [CLI v0.8.3] - 2023-07-31

- fix interpolation (should expect squared distance)

---

## [CLI v0.8.2] - 2023-07-31

- Fixes rbf interpolation functions

---

## [CLI v0.8.1] - 2023-07-29

- feat(cli): Apply subcommand now supports multiple images!
- fix(palettes): Amends for a few palettes
- feat(palettes): Added Oxocarbon
- doc: Add Link to explain LUT

---

## [CLI v0.8.0] - 2023-07-10

Functionality in the CLI has been moved entirely within subcommands now:

- `generate`: generate a hald clut for external use
- `apply`: generate or use a hald clut, and apply it to an image
- `completions`: generates shell completions

This fixes issues where the completion scripts parse the subcommand as a custom color, and suggests to type the subcommand twice.

Custom colors also now need to be separated with a `--` at the end of the command.

---

## [CLI v0.7.0] - 2023-07-08

- feat: luminosity factor for all algorithms (`--lum <factor>`)
- feat: generate completions for bash, zsh, fish, powershell, and elvish (`--completions <shell>`)
- refactor: Gaussian Sampling now uses Oklab colorspace
- refactor: exoquant dependency is fully removed

---

## [CLI v0.6.2] - 2023-07-02

- fixed Gruvbox palette
- fixed Nord palette
- fixed Rose Pine palette
- refactor: remove unnecessary floating-point casts

---

## [CLI v0.6.1] - 2023-06-15

- Quickfix: Argument conflict from shape and std. dev.
- AUR Release: lutgen-bin

---

## [CLI v0.6.0] - 2023-06-15

RBF Based algorithms now operate within the Oklab colorspace.

This is a much better perceptual colorspace that is really good for gradients between colors, and produces very consistent results in terms of luminosity and other perceptual factors.

### Breaking Changes

- `RBFRemapper` and its associated algorithm types no longer accept a generic exoquant colorspace
- CLI argument `euclide` was renamed to `shape` for GaussianRBF

---

## [CLI v0.5.0] - 2023-06-13

### New Additions

New RBF interpolation algorithms:
- Gaussian RBF
- Shepard's method (inverse distance weighting)
- Linear (1/distance) RBF

### Breaking Changes

- `interpolated_remap` module is renamed to `interpolation`
- `lutgen::generate_lut<A>` generic function has been moved into the new `GenerateLut` trait
- `InterpolatedRemapper` trait no longer contains a `new` constructor or associated type `Params`; each implementation provides their own `new` function
- `GaussianV0` struct has been removed, the V1 implementation produces identical results now
- `GaussianV1` struct has been renamed to `GaussianSampling`

---

## [CLI v0.4.3] - 2023-06-09

- unified dracula palette
- release builds compile with opt level z
- update readme example images

---

## [CLI v0.4.2] - 2023-06-02

- add Catppuccin OLED
- binary will use new minor palette versions w/o needing a new release of the main crate

---

## [CLI v0.4.1] - 2023-06-02

- Add lut cache for apply subcommand

---

## [CLI v0.4.0] - 2023-06-02

- added simple color correction algorithm for applying luts
- refactored and improved binary with new subcommand: `apply <image>`

---

## [CLI v0.3.2] - 2023-05-14

- Fix base identity algorithm

---

## [CLI v0.3.1] - 2023-05-13

- Update usage texts around the repo

---

## [CLI v0.3.0] - 2023-05-13

- Refactor the library's algorithms around generic traits and some improvements to the binary

---

## [CLI v0.2.1] - 2023-05-13

- Optimize + cleanup for lutgen-palettes

---

## [CLI v0.2.0] - 2023-05-13

Second release, with 750+ base colorschemes included in a new crate: `lutgen-palettes`!

---

## [CLI v0.1.0] - 2023-05-13

Initial release of the CLI and library. Includes palettes for catppuccin flavors.
