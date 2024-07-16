use std::collections::BTreeSet;
use std::fmt::{Debug, Display};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{stdout, IsTerminal};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;

use bpaf::{construct, long, positional, Bpaf, Doc, Parser};
use lutgen::identity::{correct_pixel, detect_level};
use lutgen::interpolation::{
    GaussianRemapper,
    GaussianSamplingRemapper,
    NearestNeighborRemapper,
    ShepardRemapper,
};
use lutgen::{GenerateLut, Image};
use lutgen_palettes::Palette;
use oklab::{srgb_to_oklab, Oklab};
use regex::{Captures, Regex};

const IMAGE_GLOB: &str = "avif bmp dds exr ff gif hdr ico jpg jpeg png pnm qoi tga tiff webp";

#[derive(Clone, Debug, Hash)]
enum DynamicPalette {
    Builtin(Palette),
    Custom(String, Vec<[u8; 3]>),
}
impl Display for DynamicPalette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicPalette::Builtin(palette) => std::fmt::Display::fmt(palette, f),
            DynamicPalette::Custom(name, _) => f.write_str(name),
        }
    }
}
impl FromStr for DynamicPalette {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some((name, path)) = Self::get_custom_palettes()
            .iter()
            .find(|(name, _)| input == name.to_lowercase())
        {
            let contents = std::fs::read_to_string(path)
                .map_err(|e| format!("failed to read custom palette file: {e}"))?;

            let mut palette = Vec::new();
            for color in contents.split_whitespace() {
                palette.push(Color::from_str(color)?.0)
            }

            Ok(DynamicPalette::Custom(name.clone(), palette))
        } else if let Ok(palette) = Palette::from_str(input) {
            Ok(DynamicPalette::Builtin(palette))
        } else {
            let mut doc: Doc = "unknown palette\n  \n  ".into();

            if let Some((_, name)) = Self::suggestions(input).pop_first() {
                doc.text("Did you mean ");
                doc.emphasis(&name);
                doc.text("\n  \n  ");
            }

            doc.emphasis("Hint: ");
            doc.text("view all palettes with ");
            doc.literal("`lutgen palette names`");
            Err(doc.monochrome(true))
        }
    }
}
impl DynamicPalette {
    const HELP: &'static str = "\
Builtin or custom palette to use.

Custom palettes can be added to `$LUTGEN_DIR` or `<CONFIG DIR>/lutgen`.
   - Linux: `/home/alice/.config/lutgen`
   - macOS: `/Users/Alice/Library/Application Support/lutgen`
   - Windows: `C:\\Users\\Alice\\AppData\\Roaming\\lutgen`

Names are case-insensitive and parsed from the file stem, minus any file extensions.
For example, `~/.config/lutgen/My-palette.txt` would be avalable to use as `my-palette`.";

    /// Argument parser and completion for palettes
    pub fn flag_parser() -> impl Parser<Self> {
        long("palette")
            .short('p')
            .argument::<String>("PALETTE")
            .help(Self::HELP)
            .complete(|v| {
                DynamicPalette::suggestions(v)
                    .into_iter()
                    .map(|(_, name)| (name, None))
                    .collect()
            })
            .parse(|s| DynamicPalette::from_str(&s))
    }

    /// Positional parser and completion for palettes
    pub fn arg_parser() -> impl Parser<Vec<Self>> {
        positional::<String>("PALETTE")
            .help(Self::HELP)
            .complete(|v| {
                DynamicPalette::suggestions(v)
                    .into_iter()
                    .map(|(_, name)| (name, None))
                    .collect()
            })
            .parse(|s| DynamicPalette::from_str(&s))
            .some("missing `all`, `names`, or at least one palette to preview")
    }

    pub fn get(&self) -> &[[u8; 3]] {
        match self {
            DynamicPalette::Builtin(p) => p.get(),
            DynamicPalette::Custom(_, p) => p.as_ref(),
        }
    }

    /// Compute a set of palette suggestions based on some input. Best matches are first in the set.
    fn suggestions(input: &str) -> BTreeSet<(u16, String)> {
        let input = input.to_lowercase();
        Palette::VARIANTS
            .iter()
            .map(ToString::to_string)
            .chain(Self::get_custom_palettes().iter().map(|v| v.0.clone()))
            .filter_map(|variant| {
                let score = strsim::jaro_winkler(&input, &variant);
                (score > 0.7).then_some((((1. - score) * 10000.) as u16, variant))
            })
            .collect::<BTreeSet<_>>()
    }

    /// Parse files in the palette directory and return all items and locations
    fn get_custom_palettes() -> Vec<(String, PathBuf)> {
        let path = std::env::var("LUTGEN_DIR")
            .map(Into::into)
            .unwrap_or(dirs::config_dir().unwrap().join("lutgen"));

        if path.is_dir() {
            std::fs::read_dir(path)
                .expect("failed to read lutgen dir")
                .map(|v| {
                    let path = v.expect("failed to get file info").path();
                    (
                        path.file_stem()
                            .expect("missing file stem")
                            .to_string_lossy()
                            .to_lowercase(),
                        path,
                    )
                })
                .collect()
        } else {
            vec![]
        }
    }
}

/// Utility for easily parsing from bpaf
#[derive(Clone, Hash)]
struct Color(pub [u8; 3]);
impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = self.0;
        f.write_str(&format!("#{r:02x}{g:02x}{b:02x}"))
    }
}
impl Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = self.0;
        f.write_str(&format!("rgb({r}, {g}, {b})"))
    }
}
impl FromStr for Color {
    type Err = String;
    fn from_str(code: &str) -> Result<Self, Self::Err> {
        // parse hex string into rgb
        let mut hex = (*code).trim_start_matches('#').to_string();
        match hex.len() {
            3 => {
                // Extend 3 character hex colors
                hex = hex.chars().flat_map(|a| [a, a]).collect();
            },
            6 => {},
            l => return Err(format!("Invalid hex length for {code}: {l}")),
        }
        if let Ok(channel_bytes) = u32::from_str_radix(&hex, 16) {
            let r = ((channel_bytes >> 16) & 0xFF) as u8;
            let g = ((channel_bytes >> 8) & 0xFF) as u8;
            let b = (channel_bytes & 0xFF) as u8;
            Ok(Self([r, g, b]))
        } else {
            Err(format!("Invalid hex color: {code}"))
        }
    }
}
impl AsRef<[u8; 3]> for Color {
    fn as_ref(&self) -> &[u8; 3] {
        &self.0
    }
}
impl Color {
    fn extra_colors() -> impl Parser<Vec<Color>> {
        positional::<String>("COLORS")
            .help("Custom colors to use. Combines with a palette if provided.")
            .strict()
            .complete(|s| {
                let hex = s.trim_start_matches('#').to_string();
                if hex.len() == 3 {
                    vec![(
                        "#".chars()
                            .chain(hex.chars().flat_map(|a| [a, a]))
                            .collect::<String>(),
                        None,
                    )]
                } else {
                    vec![(s.clone(), None)]
                }
            })
            .parse(|s| Color::from_str(&s))
            .many()
    }
}

/// Utility to wrap non-hashable types with their string impl
#[derive(Clone, Debug)]
struct Hashed<T: Clone + Debug>(pub T);
impl<T: Clone + Debug + ToString> Hash for Hashed<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_string().hash(state);
    }
}
impl<T: Clone + Debug + FromStr> FromStr for Hashed<T> {
    type Err = T::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s).map(Hashed)
    }
}
impl<T: Clone + Debug> AsRef<T> for Hashed<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}
impl<T: Clone + Debug> Display for Hashed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Bpaf, Clone, Debug, Hash)]
struct Common {
    /// Factor to multiply luminocity values by. Effectively weights the interpolation to prefer
    /// more colorful or more greyscale/unsaturated matches. Usually paired with `--preserve`.
    #[bpaf(
        short('L'),
        long("lum"),
        argument("FACTOR"),
        fallback(Hashed(1.0)),
        display_fallback
    )]
    lum_factor: Hashed<f64>,
    /// Hald clut level to generate. A level of 16 stores a value for the entire sRGB color space.
    #[bpaf(
        short,
        long,
        argument("2-16"),
        fallback(10),
        display_fallback,
        guard(|v| (2..=16).contains(v), "hald level must between 2-16"))
    ]
    level: u8,
}

#[derive(Bpaf, Clone, Debug, Hash)]
struct CommonRbf {
    /// Number of nearest colors to consider when interpolating. 0 uses all available colors.
    #[bpaf(short, long, argument("NEAREST"), fallback(16), display_fallback)]
    nearest: usize,
    /// Preserve the original image's luminocity values after interpolation.
    #[bpaf(short('P'), long, fallback(false), display_fallback)]
    preserve: bool,
}

#[derive(Bpaf, Clone, Debug, Hash)]
enum LutAlgorithm {
    // Default algorithm, so adjacent isn't used.
    GaussianRbf {
        #[bpaf(external)]
        common: Common,
        #[bpaf(external)]
        common_rbf: CommonRbf,
        /// Shape parameter for the default Gaussian RBF interpolation. Effectively creates more or
        /// less blending between colors in the palette, where bigger numbers equal less blending.
        /// Effect is heavily dependant on the number of nearest colors used.
        #[bpaf(
            short,
            long,
            argument("SHAPE"),
            fallback(Hashed(128.0)),
            display_fallback
        )]
        shape: Hashed<f64>,
    },
    #[bpaf(adjacent)]
    ShepardsMethod {
        /// Enable using Shepard's method (Inverse Distance RBF) for interpolation.
        #[bpaf(short('S'), long("shepards-method"), req_flag(()))]
        _shepards_method: (),
        /// Power parameter for shepard's method.
        #[bpaf(
            short,
            long,
            argument("POWER"),
            fallback(Hashed(4.0)),
            display_fallback
        )]
        power: Hashed<f64>,
        #[bpaf(external)]
        common_rbf: CommonRbf,
        #[bpaf(external)]
        common: Common,
    },
    #[bpaf(adjacent)]
    GaussianSampling {
        /// Enable using Gaussian sampling for interpolation (slow).
        #[bpaf(short('G'), long("gaussian-sampling"), req_flag(()))]
        _gaussian_sampling: (),
        /// Average amount of noise to apply in each iteration.
        #[bpaf(short, long, argument("MEAN"), fallback(Hashed(0.0)), display_fallback)]
        mean: Hashed<f64>,
        /// Standard deviation parameter for the noise applied in each iteration.
        #[bpaf(
            short,
            long,
            argument("STD_DEV"),
            fallback(Hashed(20.0)),
            display_fallback
        )]
        std_dev: Hashed<f64>,
        /// Number of iterations of noise to apply to each pixel.
        #[bpaf(short, long, argument("ITERS"), fallback(512), display_fallback)]
        iterations: usize,
        /// Seed for noise rng.
        #[bpaf(
            short('S'),
            long,
            argument("SEED"),
            fallback(42080085),
            display_fallback
        )]
        seed: u64,
        #[bpaf(external)]
        common: Common,
    },
    #[bpaf(adjacent)]
    NearestNeighbor {
        /// Disable interpolation completely.
        #[bpaf(short('N'), long("nearest-neighbor"), req_flag(()))]
        _nearest_neighbor: (),
        #[bpaf(external)]
        common: Common,
    },
    #[bpaf(skip)]
    HaldClut { file: PathBuf },
}

/// Manually allow using an external hald clut, hack since we dont want to allow this for generate,
/// but we do for apply.
fn hald_clut_or_algorithm() -> impl Parser<LutAlgorithm> {
    let clut = long("hald-clut")
        .help("External Hald CLUT to use instead of generating one.")
        .argument::<PathBuf>("FILE")
        .map(|file| LutAlgorithm::HaldClut { file });
    construct!([clut, lut_algorithm()])
}

impl LutAlgorithm {
    fn generate(&self, name: &str, colors: Vec<[u8; 3]>) -> Result<Image, String> {
        if let Self::HaldClut { file } = &self {
            return load_image(file);
        }

        let time = Instant::now();

        let lut = match self {
            LutAlgorithm::GaussianRbf {
                shape,
                common_rbf: CommonRbf { nearest, preserve },
                common: Common { level, lum_factor },
                ..
            } => GaussianRemapper::new(&colors, shape.0, *nearest, lum_factor.0, *preserve)
                .generate_lut(*level),
            LutAlgorithm::ShepardsMethod {
                power,
                common_rbf: CommonRbf { nearest, preserve },
                common: Common { level, lum_factor },
                ..
            } => ShepardRemapper::new(&colors, power.0, *nearest, lum_factor.0, *preserve)
                .generate_lut(*level),
            LutAlgorithm::GaussianSampling {
                mean,
                std_dev,
                iterations,
                seed,
                common: Common { level, lum_factor },
                ..
            } => GaussianSamplingRemapper::new(
                &colors,
                mean.0,
                std_dev.0,
                *iterations,
                lum_factor.0,
                *seed,
            )
            .generate_lut(*level),
            LutAlgorithm::NearestNeighbor {
                common: Common { level, lum_factor },
                ..
            } => NearestNeighborRemapper::new(&colors, lum_factor.0).generate_lut(*level),
            _ => unreachable!(),
        };
        println!("✔ Generated \"{name}\" LUT in {:.2?}", time.elapsed());

        Ok(lut)
    }
}

#[derive(Bpaf, Clone, Debug, Hash)]
enum PaletteArgs {
    /// Print all palette names. Useful for scripting and searching.
    #[bpaf(command)]
    Names,
    /// Print all palette names and colors.
    #[bpaf(command)]
    All,
    Palettes(
        /// Palettes to print color previews for.
        #[bpaf(external(DynamicPalette::arg_parser))]
        Vec<DynamicPalette>,
    ),
}

/// Concat an optional palette and extra colors, as well as constructing a name tag.
fn concat_colors(
    palette: Option<DynamicPalette>,
    extra_colors: Vec<Color>,
) -> Result<(String, Vec<[u8; 3]>), String> {
    let mut name = String::new();
    let mut colors = palette
        .as_ref()
        .map(|p| {
            name.push_str(&p.to_string());
            p.get().to_vec()
        })
        .unwrap_or_default();
    if !extra_colors.is_empty() {
        if !name.is_empty() {
            name.push('-');
        }
        name.push_str("custom");
        colors.extend(extra_colors.iter().map(AsRef::as_ref));
    }

    if !colors.is_empty() {
        Ok((name, colors))
    } else {
        Err("A palette (-p/--palette) and/or custom colors (-- #FFFFFF) are required".into())
    }
}

#[derive(Bpaf, Clone, Debug, Hash)]
#[bpaf(
    options,
    version,
    max_width(120),
    descr(env!("CARGO_PKG_DESCRIPTION")),
    header({
        let mut doc = Doc::default();
        doc.emphasis("Examples:");
        doc.text("\n  $ ");
        doc.literal("lutgen generate -p gruvbox-dark");
        doc.text("\n  $ ");
        doc.literal("lutgen apply -p carburetor wallpaper.png");
        doc.text("\n  $ ");
        doc.literal("lutgen patch -Np tomorrow theme.css > tomorrow.diff");
        doc.text("\n  $ ");
        doc.literal("lutgen palette gruvbox-dark gruvbox-light");
        doc
    }),
    footer({
        let mut doc = Doc::default();
        doc.emphasis("Supported image formats:");
        doc.text("\n");
        for extension in IMAGE_GLOB.split(' ') {
            doc.text(" ");
            doc.literal(extension);
        };
        doc
    }),
)]
enum Lutgen {
    /// Generate and save a Hald CLUT to disk.
    #[bpaf(command, short('g'))]
    Generate {
        /// Path to write output to.
        #[bpaf(short, long, argument("PATH"), complete_shell(bpaf::ShellComp::Dir { mask: None }))]
        output: Option<PathBuf>,
        #[bpaf(optional, external(DynamicPalette::flag_parser))]
        palette: Option<DynamicPalette>,
        #[bpaf(external)]
        lut_algorithm: LutAlgorithm,
        #[bpaf(external(Color::extra_colors))]
        extra_colors: Vec<Color>,
    },
    /// Apply a generated or provided Hald CLUT to images.
    #[bpaf(command, short('a'))]
    Apply {
        /// Always save to a directory when there is only one input file.
        /// (matches output behavior for multiple files)
        #[bpaf(short, long)]
        dir: bool,
        /// Path to write output to.
        #[bpaf(short, long, argument("PATH"), complete_shell(bpaf::ShellComp::Dir { mask: None }))]
        output: Option<PathBuf>,
        #[bpaf(optional, external(DynamicPalette::flag_parser))]
        palette: Option<DynamicPalette>,
        /// Cache generated LUT. No effect when using an external LUT.
        #[bpaf(short, long)]
        cache: bool,
        #[bpaf(external)]
        hald_clut_or_algorithm: LutAlgorithm,
        /// Images to correct, using the generated or provided hald clut.
        #[bpaf(
            positional("IMAGES"),
            guard(|v| v.exists(), "No such file or directory"),
            some("At least one image is needed to apply"),
        )]
        input: Vec<PathBuf>,
        #[bpaf(external(Color::extra_colors))]
        extra_colors: Vec<Color>,
    },
    /// Generate a patch for colors inside text files.
    #[bpaf(command, short('p'))]
    Patch {
        /// Write changes directly to the files.
        #[bpaf(short, long)]
        write: bool,
        /// Disable computing and printing the patch. Usually paired with --write.
        #[bpaf(short, long)]
        no_patch: bool,
        #[bpaf(optional, external(DynamicPalette::flag_parser))]
        palette: Option<DynamicPalette>,
        #[bpaf(external)]
        hald_clut_or_algorithm: LutAlgorithm,
        /// Text files to generate patches for.
        #[bpaf(
            positional::<PathBuf>("FILES"),
            guard(|path| path.exists(), "No such file or directory"),
            parse(|path| std::fs::read_to_string(&path).map(|v| (path, v))),
            some("At least one file is needed to patch")
        )]
        input: Vec<(PathBuf, String)>,
        #[bpaf(external(Color::extra_colors))]
        extra_colors: Vec<Color>,
    },
    /// Print palette names and colors
    #[bpaf(
        command,
        short('P'),
        header({
            let mut doc = Doc::default();
            doc.emphasis("Examples:");
            doc.text("\n  $ ");
            doc.literal("lutgen palette all");
            doc.text("\n  $ ");
            doc.literal("lutgen palette names | grep gruvbox");
            doc.text("\n  $ ");
            doc.literal("lutgen palette oxocarbon-dark oxocarbon-light");
            doc.text("\n  $ ");
            doc.literal("lutgen palette carburetor > palette.txt");
            doc
        })
    )]
    Palette(#[bpaf(external(palette_args))] PaletteArgs),
}

fn load_image<P: AsRef<Path>>(path: P) -> Result<Image, String> {
    let path = path.as_ref();
    let time = Instant::now();
    let lut = image::io::Reader::open(path)
        .map_err(|e| format!("failed to open image: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("failed to guess image format: {e}"))?
        .decode()
        .map_err(|e| format!("failed to decode image: {e}"))?
        .to_rgb8();
    println!("✔ Loaded {path:?} in {:.2?}", time.elapsed());
    Ok(lut)
}

impl Lutgen {
    fn execute(self) -> Result<String, String> {
        match self {
            Lutgen::Generate {
                output,
                palette,
                lut_algorithm,
                extra_colors,
            } => Lutgen::generate(output, palette, lut_algorithm, extra_colors),
            Lutgen::Apply {
                cache,
                ref hald_clut_or_algorithm,
                ..
            } => {
                let hash =
                    if cache && !matches!(hald_clut_or_algorithm, LutAlgorithm::HaldClut { .. }) {
                        let mut hasher = DefaultHasher::new();
                        self.hash(&mut hasher);
                        Some(hasher.finish())
                    } else {
                        None
                    };

                let Lutgen::Apply {
                    dir,
                    output,
                    palette,
                    hald_clut_or_algorithm,
                    input,
                    extra_colors,
                    ..
                } = self
                else {
                    unreachable!()
                };

                Lutgen::apply(
                    hash,
                    dir,
                    output,
                    palette,
                    hald_clut_or_algorithm,
                    input,
                    extra_colors,
                )
            },
            Lutgen::Patch {
                write,
                no_patch,
                palette,
                hald_clut_or_algorithm,
                input,
                extra_colors,
            } => Lutgen::patch(
                write,
                no_patch,
                palette,
                hald_clut_or_algorithm,
                input,
                extra_colors,
            ),
            Lutgen::Palette(args) => Lutgen::palette(args),
        }
    }

    fn generate(
        output: Option<PathBuf>,
        palette: Option<DynamicPalette>,
        lut_algorithm: LutAlgorithm,
        extra_colors: Vec<Color>,
    ) -> Result<String, String> {
        let (name, colors) = concat_colors(palette, extra_colors)?;
        let lut = lut_algorithm.generate(&name, colors)?;
        let time = Instant::now();
        let path = output.unwrap_or(format!("{name}.png").into());
        lut.save(&path).map_err(|e| e.to_string())?;
        println!("✔ Saved output to {path:?} in {:.2?}", time.elapsed());
        Ok("generating ".into())
    }

    fn apply(
        hash: Option<u64>,
        dir: bool,
        output: Option<PathBuf>,
        palette: Option<DynamicPalette>,
        hald_clut_or_algorithm: LutAlgorithm,
        input: Vec<PathBuf>,
        extra_colors: Vec<Color>,
    ) -> Result<String, String> {
        let (name, colors) = concat_colors(palette, extra_colors)?;

        let lut = if let Some(hash) = hash {
            let mut path = dirs::cache_dir()
                .expect("failed to determine cache dir")
                .join("lutgen");
            if !path.exists() {
                std::fs::create_dir_all(&path).expect("failed to create cache directory");
            }

            path = path.join(format!("{name}-{hash}.png"));
            if !path.exists() {
                let lut = hald_clut_or_algorithm.generate(&name, colors)?;
                let time = Instant::now();
                lut.save(path)
                    .map_err(|e| format!("failed to write cached LUT: {e}"))?;
                println!("✔ Cached \"{name}\" LUT in {:.02?}", time.elapsed());
                lut
            } else {
                load_image(path)?
            }
        } else {
            hald_clut_or_algorithm.generate(&name, colors)?
        };

        for file in &input {
            let mut image = load_image(file)?;

            let time = Instant::now();
            lutgen::identity::correct_image(&mut image, &lut);
            println!("✔ Applied LUT to {file:?} in {:.2?}", time.elapsed());

            let time = Instant::now();
            let path = if input.len() > 1 {
                // For multiple images, the output path is always treated as a directory
                let path = output.clone().unwrap_or(PathBuf::from(&name));
                if !path.exists() {
                    std::fs::create_dir_all(&path).expect("failed to create output directory");
                }
                path.join(file.file_name().unwrap())
            } else {
                // For single images
                match &output {
                    // If user provided a path
                    Some(path) => {
                        if dir {
                            // The path is always a dir
                            if !path.exists() {
                                std::fs::create_dir_all(path)
                                    .expect("failed to create output directory");
                            }
                            path.join(file.file_name().unwrap())
                        } else {
                            // Otherwise, ensure the parent dir exists
                            if let Some(parent) = path.parent() {
                                if !parent.exists() {
                                    std::fs::create_dir_all(parent)
                                        .expect("failed to create output directory");
                                }
                            }

                            if path.is_dir() {
                                // Enable dir mode if user supplied an existing directory
                                path.join(file.file_name().unwrap())
                            } else {
                                path.clone()
                            }
                        }
                    },
                    None => {
                        if dir {
                            // always create a palette dir
                            let path = PathBuf::from(&name);
                            if !path.exists() {
                                std::fs::create_dir_all(&path)
                                    .expect("failed to create output directory");
                            }
                            path.join(file.file_name().unwrap())
                        } else {
                            // create an image adjacent to the original, with a palette name prefix
                            let mut file_name =
                                file.file_stem().unwrap().to_string_lossy().to_string();
                            file_name.push('_');
                            file_name.push_str(&name);
                            let extension = file
                                .extension()
                                .map(|s| s.to_string_lossy())
                                .unwrap_or("png".into());

                            let mut path = file.clone();
                            path.set_file_name(file_name);
                            path.set_extension(extension.as_ref());
                            path
                        }
                    },
                }
            };
            image
                .save(&path)
                .map_err(|e| format!("failed to write image: {e}"))?;
            println!("✔ Saved output to {path:?} in {:.2?}", time.elapsed());
        }

        Ok(format!(
            "applying {} file{} ",
            input.len(),
            if input.len() > 1 { "s" } else { "" }
        ))
    }

    fn patch(
        write: bool,
        no_patch: bool,
        palette: Option<DynamicPalette>,
        hald_clut_or_algorithm: LutAlgorithm,
        input: Vec<(PathBuf, String)>,
        extra_colors: Vec<Color>,
    ) -> Result<String, String> {
        const REGEX: &str = r"(#)([0-9a-fA-F]{3}){1,2}|(rgb)\(((?:[0-9\s]+,?){3})\)|(rgba)\(((?:[0-9\s]+,?){3}),([\s0-9.]*)\)";

        let (name, colors) = concat_colors(palette, extra_colors)?;
        let lut = hald_clut_or_algorithm.generate(&name, colors)?;
        let level = detect_level(&lut);

        let len = input.len();
        let re = Regex::new(REGEX).expect("failed to build regex");
        for (file, contents) in input {
            let time = Instant::now();

            let counter = &mut 0u32;
            let replaced = re.replace_all(&contents, |caps: &Captures| {
                *counter += 1;
                if caps.get(1).is_some() {
                    let rgb = Color::from_str(&caps[2]).expect("valid hex");
                    let [r, g, b] = correct_pixel(rgb.as_ref(), &lut, level);
                    format!("#{r:02x}{g:02x}{b:02x}")
                } else if caps.get(3).is_some() {
                    let inner: Vec<u8> = caps[4]
                        .split(',')
                        .map(|s| s.trim().parse().expect("invalid rgb code"))
                        .collect();
                    let [r, g, b] = correct_pixel(&[inner[0], inner[1], inner[2]], &lut, level);
                    format!("rgb({r}, {g}, {b})")
                } else if caps.get(5).is_some() {
                    let inner: Vec<u8> = caps[6]
                        .split(',')
                        .map(|s| s.trim().parse().expect("invalid rgb point"))
                        .collect();
                    let [r, g, b] = correct_pixel(&[inner[0], inner[1], inner[2]], &lut, level);
                    format!("rgba({r}, {g}, {b}, {})", &caps[7].trim())
                } else {
                    unreachable!()
                }
            });

            eprintln!(
                "✔ Replaced {counter} colors in {file:?} in {:?}",
                time.elapsed()
            );

            if *counter > 0 {
                let replaced = replaced.to_string();
                if write {
                    std::fs::write(&file, &replaced).expect("failed to write file");
                    eprintln!("✔ Wrote changes to {file:?}");
                }

                if !no_patch {
                    // Compute and print diff for the file
                    let time = Instant::now();
                    let input = imara_diff::intern::InternedInput::new(
                        contents.as_str(),
                        replaced.as_str(),
                    );
                    let diff = imara_diff::diff(
                        imara_diff::Algorithm::Histogram,
                        &input,
                        imara_diff::UnifiedDiffBuilder::new(&input),
                    );

                    println!(
                        "--- a/{file}\n+++ b/{file}\n{diff}\n",
                        file = file.to_string_lossy()
                    );
                    eprintln!("✔ Computed diff in {:.2?}", time.elapsed());
                }
            }

            // Free up memory in case it's not right away
            drop(contents);
        }

        Ok(format!(
            "patching {len} file{} ",
            if len > 1 { "s" } else { "" },
        ))
    }

    fn palette(args: PaletteArgs) -> Result<String, String> {
        if matches!(args, PaletteArgs::Names) {
            Palette::VARIANTS.iter().for_each(|p| println!("{p}"));
            return Ok(Default::default());
        }

        let is_terminal = stdout().is_terminal();
        let print = move |palette: DynamicPalette| {
            // Print palette name with underline
            if is_terminal {
                eprintln!("\n\x1b[4m{palette}\x1b[0m\n");
            }
            for color in palette.get() {
                let color = Color(*color);
                if is_terminal {
                    // Set background to the color, and choose foreground based on luminocity
                    let [r, g, b] = color.0;
                    let Oklab { l, .. } = srgb_to_oklab(color.0.into());
                    let fg = if l < 0.5 {
                        "\x1b[38;2;255;255;255m"
                    } else {
                        "\x1b[38;2;0;0;0m"
                    };
                    println!("\x1b[48;2;{r};{g};{b}m{fg}{color}\x1b[0m");
                } else {
                    println!("{color}");
                }
            }
        };

        match args {
            PaletteArgs::All => Palette::VARIANTS
                .iter()
                .map(|&p| DynamicPalette::Builtin(p))
                .for_each(print),
            PaletteArgs::Palettes(palettes) => {
                palettes.into_iter().for_each(print);
            },
            _ => unreachable!(),
        }

        Ok("".into())
    }
}

fn main() {
    let time = Instant::now();
    match lutgen().fallback_to_usage().run().execute() {
        Ok(s) => eprintln!("\nFinished {s}in {:.2?}", time.elapsed()),
        Err(e) => {
            bpaf::ParseFailure::Stderr(e.as_str().into()).print_mesage(80);
            std::process::exit(1)
        },
    }
}

#[cfg(test)]
#[test]
fn generate_docs() {
    let app = env!("CARGO_PKG_NAME");
    let options = lutgen();

    let roff = options.render_manpage(app, bpaf::doc::Section::General, None, None, None);
    std::fs::write("docs/lutgen.1", roff).expect("failed to write manpage");

    let md = options.header("").render_markdown(app);
    std::fs::write("docs/README.md", md).expect("failed to write markdown docs");
}
