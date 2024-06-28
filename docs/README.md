


# Command summary

  * [`lutgen`↴](#lutgen)
  * [`lutgen generate`↴](#lutgen-generate)
  * [`lutgen apply`↴](#lutgen-apply)
  * [`lutgen patch`↴](#lutgen-patch)
  * [`lutgen palette`↴](#lutgen-palette)
  * [`lutgen palette names`↴](#lutgen-palette-names)
  * [`lutgen palette all`↴](#lutgen-palette-all)

## lutgen

LUT Generator

**Usage**: **`lutgen`** _`COMMAND ...`_



**Available options:**
- **`-h`**, **`--help`** &mdash; 
  Prints help information
- **`-V`**, **`--version`** &mdash; 
  Prints version information



**Available commands:**
- **`generate`**, **`g`** &mdash; 
  Generate and save a Hald CLUT to disk.
- **`apply`**, **`a`** &mdash; 
  Apply a generated or provided Hald CLUT to images.
- **`patch`**, **`p`** &mdash; 
  Generate a patch for colors inside text files.
- **`palette`**, **`P`** &mdash; 
  Print palette names and colors



**Supported image formats:**
 avif bmp dds exr ff gif hdr ico jpeg png pnm qoi tga tiff webp


## lutgen generate

Generate and save a Hald CLUT to disk.

**Usage**: **`lutgen`** **`generate`** \[**`-o`**=_`PATH`_\] \[**`-p`**=_`PALETTE`_\] (\[**`-s`**=_`SHAPE`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-S`** \[**`-p`**=_`POWER`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-G`** \[**`-m`**=_`MEAN`_\] \[**`-s`**=_`STD_DEV`_\] \[**`-i`**=_`ITERS`_\] \[**`-S`**=_`SEED`_\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-N`** \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]) **`--`** \[_`COLORS`_\]...

**Available positional items:**
- _`COLORS`_ &mdash; 
  Custom colors to use. Combines with a palette if provided.



**Available options:**
- **`-o`**, **`--output`**=_`PATH`_ &mdash; 
  Path to write output to.
- **`-p`**, **`--palette`**=_`PALETTE`_ &mdash; 
  Palette to use (`lutgen palette names` to view all options).
- **`-s`**, **`--shape`**=_`SHAPE`_ &mdash; 
  Shape parameter for the default Gaussian RBF interpolation. Effectively creates more or less blending between colors in the palette, where bigger numbers equal less blending. Effect is heavily dependant on the number of nearest colors used.
   
  [default: 128]
- **`-n`**, **`--nearest`**=_`NEAREST`_ &mdash; 
  Number of nearest colors to consider when interpolating. 0 uses all available colors.
   
  [default: 16]
- **`-P`**, **`--preserve`** &mdash; 
  Preserve the original image's luminocity values after interpolation.
   
  [default: false]
- **`-L`**, **`--lum`**=_`FACTOR`_ &mdash; 
  Factor to multiply luminocity values by. Effectively weights the interpolation to prefer more colorful or more greyscale/unsaturated matches. Usually paired with `--preserve`.
   
  [default: 1]
- **`-l`**, **`--level`**=_`2-16`_ &mdash; 
  Hald clut level to generate. A level of 16 stores a value for the entire sRGB color space.
   
  [default: 10]
### **`-S`** \[**`-p`**=_`POWER`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-S`**, **`--shepards-method`** &mdash; 
  Enable using Shepard's method (Inverse Distance RBF) for interpolation.
- **`-p`**, **`--power`**=_`POWER`_ &mdash; 
  Power parameter for shepard's method.
   
  [default: 4]


### **`-G`** \[**`-m`**=_`MEAN`_\] \[**`-s`**=_`STD_DEV`_\] \[**`-i`**=_`ITERS`_\] \[**`-S`**=_`SEED`_\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-G`**, **`--gaussian-sampling`** &mdash; 
  Enable using Gaussian sampling for interpolation (slow).
- **`-m`**, **`--mean`**=_`MEAN`_ &mdash; 
  Average amount of noise to apply in each iteration.
   
  [default: 0]
- **`-s`**, **`--std-dev`**=_`STD_DEV`_ &mdash; 
  Standard deviation parameter for the noise applied in each iteration.
   
  [default: 20]
- **`-i`**, **`--iterations`**=_`ITERS`_ &mdash; 
  Number of iterations of noise to apply to each pixel.
   
  [default: 512]
- **`-S`**, **`--seed`**=_`SEED`_ &mdash; 
  Seed for noise rng.
   
  [default: 42080085]


### **`-N`** \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-N`**, **`--nearest-neighbor`** &mdash; 
  Disable interpolation completely.


- **`-h`**, **`--help`** &mdash; 
  Prints help information


## lutgen apply

Apply a generated or provided Hald CLUT to images.

**Usage**: **`lutgen`** **`apply`** \[**`-o`**=_`PATH`_\] \[**`-p`**=_`PALETTE`_\] (\[**`-s`**=_`SHAPE`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-S`** \[**`-p`**=_`POWER`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-G`** \[**`-m`**=_`MEAN`_\] \[**`-s`**=_`STD_DEV`_\] \[**`-i`**=_`ITERS`_\] \[**`-S`**=_`SEED`_\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-N`** \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`--hald-clut`**=_`FILE`_) _`IMAGES`_... **`--`** \[_`COLORS`_\]...

**Available positional items:**
- _`IMAGES`_ &mdash; 
  Images to correct, using the generated or provided hald clut.
- _`COLORS`_ &mdash; 
  Custom colors to use. Combines with a palette if provided.



**Available options:**
- **`-o`**, **`--output`**=_`PATH`_ &mdash; 
  Path to write output to.
- **`-p`**, **`--palette`**=_`PALETTE`_ &mdash; 
  Palette to use (`lutgen palette names` to view all options).
- **`-s`**, **`--shape`**=_`SHAPE`_ &mdash; 
  Shape parameter for the default Gaussian RBF interpolation. Effectively creates more or less blending between colors in the palette, where bigger numbers equal less blending. Effect is heavily dependant on the number of nearest colors used.
   
  [default: 128]
- **`-n`**, **`--nearest`**=_`NEAREST`_ &mdash; 
  Number of nearest colors to consider when interpolating. 0 uses all available colors.
   
  [default: 16]
- **`-P`**, **`--preserve`** &mdash; 
  Preserve the original image's luminocity values after interpolation.
   
  [default: false]
- **`-L`**, **`--lum`**=_`FACTOR`_ &mdash; 
  Factor to multiply luminocity values by. Effectively weights the interpolation to prefer more colorful or more greyscale/unsaturated matches. Usually paired with `--preserve`.
   
  [default: 1]
- **`-l`**, **`--level`**=_`2-16`_ &mdash; 
  Hald clut level to generate. A level of 16 stores a value for the entire sRGB color space.
   
  [default: 10]
### **`-S`** \[**`-p`**=_`POWER`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-S`**, **`--shepards-method`** &mdash; 
  Enable using Shepard's method (Inverse Distance RBF) for interpolation.
- **`-p`**, **`--power`**=_`POWER`_ &mdash; 
  Power parameter for shepard's method.
   
  [default: 4]


### **`-G`** \[**`-m`**=_`MEAN`_\] \[**`-s`**=_`STD_DEV`_\] \[**`-i`**=_`ITERS`_\] \[**`-S`**=_`SEED`_\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-G`**, **`--gaussian-sampling`** &mdash; 
  Enable using Gaussian sampling for interpolation (slow).
- **`-m`**, **`--mean`**=_`MEAN`_ &mdash; 
  Average amount of noise to apply in each iteration.
   
  [default: 0]
- **`-s`**, **`--std-dev`**=_`STD_DEV`_ &mdash; 
  Standard deviation parameter for the noise applied in each iteration.
   
  [default: 20]
- **`-i`**, **`--iterations`**=_`ITERS`_ &mdash; 
  Number of iterations of noise to apply to each pixel.
   
  [default: 512]
- **`-S`**, **`--seed`**=_`SEED`_ &mdash; 
  Seed for noise rng.
   
  [default: 42080085]


### **`-N`** \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-N`**, **`--nearest-neighbor`** &mdash; 
  Disable interpolation completely.


- **`    --hald-clut`**=_`FILE`_ &mdash; 
  External Hald CLUT to use
- **`-h`**, **`--help`** &mdash; 
  Prints help information


## lutgen patch

Generate a patch for colors inside text files.

**Usage**: **`lutgen`** **`patch`** \[**`-w`**\] \[**`-n`**\] \[**`-p`**=_`PALETTE`_\] (\[**`-s`**=_`SHAPE`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-S`** \[**`-p`**=_`POWER`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-G`** \[**`-m`**=_`MEAN`_\] \[**`-s`**=_`STD_DEV`_\] \[**`-i`**=_`ITERS`_\] \[**`-S`**=_`SEED`_\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`-N`** \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\] | **`--hald-clut`**=_`FILE`_) _`FILES`_... **`--`** \[_`COLORS`_\]...

**Available positional items:**
- _`FILES`_ &mdash; 
  Text files to generate patches for.
- _`COLORS`_ &mdash; 
  Custom colors to use. Combines with a palette if provided.



**Available options:**
- **`-w`**, **`--write`** &mdash; 
  Enable writing changes directly to the files.
   
  [default: false]
- **`-n`**, **`--no-patch`** &mdash; 
  Disable computing and printing the patch. Usually paired with --write.
   
  [default: false]
- **`-p`**, **`--palette`**=_`PALETTE`_ &mdash; 
  Palette to use (`lutgen palette names` to view all options).
- **`-s`**, **`--shape`**=_`SHAPE`_ &mdash; 
  Shape parameter for the default Gaussian RBF interpolation. Effectively creates more or less blending between colors in the palette, where bigger numbers equal less blending. Effect is heavily dependant on the number of nearest colors used.
   
  [default: 128]
- **`-n`**, **`--nearest`**=_`NEAREST`_ &mdash; 
  Number of nearest colors to consider when interpolating. 0 uses all available colors.
   
  [default: 16]
- **`-P`**, **`--preserve`** &mdash; 
  Preserve the original image's luminocity values after interpolation.
   
  [default: false]
- **`-L`**, **`--lum`**=_`FACTOR`_ &mdash; 
  Factor to multiply luminocity values by. Effectively weights the interpolation to prefer more colorful or more greyscale/unsaturated matches. Usually paired with `--preserve`.
   
  [default: 1]
- **`-l`**, **`--level`**=_`2-16`_ &mdash; 
  Hald clut level to generate. A level of 16 stores a value for the entire sRGB color space.
   
  [default: 10]
### **`-S`** \[**`-p`**=_`POWER`_\] \[**`-n`**=_`NEAREST`_\] \[**`-P`**\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-S`**, **`--shepards-method`** &mdash; 
  Enable using Shepard's method (Inverse Distance RBF) for interpolation.
- **`-p`**, **`--power`**=_`POWER`_ &mdash; 
  Power parameter for shepard's method.
   
  [default: 4]


### **`-G`** \[**`-m`**=_`MEAN`_\] \[**`-s`**=_`STD_DEV`_\] \[**`-i`**=_`ITERS`_\] \[**`-S`**=_`SEED`_\] \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-G`**, **`--gaussian-sampling`** &mdash; 
  Enable using Gaussian sampling for interpolation (slow).
- **`-m`**, **`--mean`**=_`MEAN`_ &mdash; 
  Average amount of noise to apply in each iteration.
   
  [default: 0]
- **`-s`**, **`--std-dev`**=_`STD_DEV`_ &mdash; 
  Standard deviation parameter for the noise applied in each iteration.
   
  [default: 20]
- **`-i`**, **`--iterations`**=_`ITERS`_ &mdash; 
  Number of iterations of noise to apply to each pixel.
   
  [default: 512]
- **`-S`**, **`--seed`**=_`SEED`_ &mdash; 
  Seed for noise rng.
   
  [default: 42080085]


### **`-N`** \[**`-L`**=_`FACTOR`_\] \[**`-l`**=_`2-16`_\]
- **`-N`**, **`--nearest-neighbor`** &mdash; 
  Disable interpolation completely.


- **`    --hald-clut`**=_`FILE`_ &mdash; 
  External Hald CLUT to use
- **`-h`**, **`--help`** &mdash; 
  Prints help information


## lutgen palette

Print palette names and colors

**Usage**: **`lutgen`** **`palette`** (_`COMMAND ...`_ | _`PALETTE`_...)

**Examples:**
 $ lutgen palette carburetor > carburetor.txt
 $ lutgen palette all
 $ lutgen palette names | grep gruvbox

**Available positional items:**
- _`PALETTE`_ &mdash; 
  Palette to use (`lutgen palette names` to view all options).



**Available options:**
- **`-h`**, **`--help`** &mdash; 
  Prints help information



**Available commands:**
- **`names`** &mdash; 
  Print all palette names. Useful for scripting and searching.
- **`all`** &mdash; 
  Print all palette names and colors.


## lutgen palette names

Print all palette names. Useful for scripting and searching.

**Usage**: **`lutgen`** **`palette`** **`names`** 

**Available options:**
- **`-h`**, **`--help`** &mdash; 
  Prints help information


## lutgen palette all

Print all palette names and colors.

**Usage**: **`lutgen`** **`palette`** **`all`** 

**Available options:**
- **`-h`**, **`--help`** &mdash; 
  Prints help information


