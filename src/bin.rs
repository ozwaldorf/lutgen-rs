use std::{
    fs::create_dir_all,
    io::{stdout, Write},
    path::{Path, PathBuf},
    process::exit,
    time::Instant,
};

use clap::{
    arg, command,
    error::{ContextKind, ContextValue, ErrorKind},
    CommandFactory, Parser, ValueEnum,
};
use clap_complete::{generate, Shell};
use dirs::cache_dir;
use image::io::Reader;
use lutgen::{
    identity::{self, correct_pixel},
    interpolation::*,
    GenerateLut, Image,
};
use lutgen_palettes::Palette;
use oklab::{srgb_to_oklab, Oklab};
use regex::{Captures, Regex};
use spinners::{Spinner, Spinners};

const SEED: u64 = u64::from_be_bytes(*b"42080085");
const REGEX: &str = r"(#)([0-9a-fA-F]{3}){1,2}|(rgb)\(((?:[0-9\s]+,?){3})\)|(rgba)\(((?:[0-9\s]+,?){3}),([\s0-9.]*)\)";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct BinArgs {
    #[command(subcommand)]
    subcommand: Subcommands,
}

#[derive(Parser, Debug)]
enum Subcommands {
    /// Generate a hald clut for external or manual usage.
    Generate {
        #[clap(flatten)]
        lut_args: LutArgs,
        /// Path to write output to.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Correct an image using a hald clut, either generating it, or loading it externally.
    Apply {
        /// Image(s) to correct with a hald clut.
        #[arg(required = true)]
        images: Vec<PathBuf>,
        /// Optional path to write output to. For multiple files, the output will be under a
        /// folder.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// An external hald-clut to use. Conflicts with all lut generation related arguments.
        #[arg(
            long,
            conflicts_with = "palette",
            conflicts_with = "level",
            conflicts_with = "algorithm",
            conflicts_with = "nearest",
            conflicts_with = "shape",
            conflicts_with = "power",
            conflicts_with = "mean",
            conflicts_with = "std_dev",
            conflicts_with = "iterations",
            conflicts_with = "cache",
            conflicts_with = "force",
            conflicts_with = "luminosity",
            conflicts_with = "preserve"
        )]
        hald_clut: Option<PathBuf>,
        /// Enable caching the generated LUT.
        #[arg(short, long, default_value_t = false)]
        cache: bool,
        /// Force overwriting the cached LUT.
        #[arg(short, long, default_value_t = false, requires = "cache")]
        force: bool,
        #[clap(flatten)]
        lut_args: LutArgs,
    },
    /// Generate a patch for rgb colors inside text files.
    Patch {
        /// Text files to replace rgb colors in.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Write output directly instead of generating a diff.
        #[arg(short, long, default_value_t = false)]
        write: bool,
        #[clap(flatten)]
        lut_args: LutArgs,
    },
    /// Print palette colors and names.
    Palette {
        /// Palette to print. If none, all palettes will be printed.
        palette: Option<Palette>,
        /// When printing all palettes, only print the names and no colors.
        /// Useful for scripting batch palette operations, and grepping for palettes.
        #[arg(short, long)]
        name_only: bool,
    },
    /// Generate shell completions.
    Completions { shell: Shell },
}

#[derive(Parser, Debug)]
struct LutArgs {
    /// Custom hexadecimal colors to add to the palette.
    /// If `-p` is not used to specify a base palette, at least 1 color is required.
    #[arg(last = true)]
    custom_colors: Vec<String>,
    /// Predefined popular color palettes. Use `lutgen -p` to view all options. Compatible with
    /// custom colors.
    #[arg(short, long, value_enum, hide_possible_values = true)]
    palette: Option<Palette>,
    /// Hald level (ex: 8 = 512x512 image)
    #[arg(short, long, default_value_t = 8)]
    level: u8,
    /// Algorithm to remap the LUT with.
    #[arg(short, long, value_enum, default_value = "gaussian-rbf")]
    algorithm: Algorithm,
    /// Luminosity factor for all algorithms. Used for weighting the luminosity vs color channels
    /// when computing color distances.
    ///
    /// Factors greater than 1 will result in more "greyscale" colors, and factors less than 1
    /// provide a more colorful hald clut.
    #[arg(long = "lum", default_value_t = 1.0)]
    luminosity: f64,
    /// Preserve the original luminosity values for the output colors for RBF based algorithms. The
    /// luminosity factor is still used for distance computations.
    #[arg(long, default_value_t = false)]
    preserve: bool,
    /// Number of nearest palette colors to consider at any given time for RBF based algorithms.
    /// 0 uses unlimited (all) colors.
    #[arg(
        short,
        long,
        default_value_t = 16,
        conflicts_with = "mean",
        conflicts_with = "std_dev",
        conflicts_with = "iterations"
    )]
    nearest: usize,
    /// Gaussian RBF's shape parameter.
    /// Higher values will mix colors less, lower values mix colors more.
    #[arg(
        short,
        long,
        default_value_t = 128.0,
        conflicts_with = "power",
        conflicts_with = "mean",
        conflicts_with = "std_dev",
        conflicts_with = "iterations"
    )]
    shape: f64,
    /// Shepard algorithm's power parameter.
    #[arg(
        long,
        default_value_t = 4.0,
        conflicts_with = "shape",
        conflicts_with = "mean",
        conflicts_with = "std_dev",
        conflicts_with = "iterations"
    )]
    power: f64,
    /// Gaussian sampling algorithm's mean parameter.
    #[arg(
        short,
        long,
        default_value_t = 0.0,
        conflicts_with = "shape",
        conflicts_with = "power",
        conflicts_with = "nearest",
        conflicts_with = "preserve"
    )]
    mean: f64,
    /// Gaussian sampling algorithm's standard deviation parameter.
    #[arg(
        long,
        default_value_t = 20.0,
        conflicts_with = "shape",
        conflicts_with = "power",
        conflicts_with = "nearest",
        conflicts_with = "preserve"
    )]
    std_dev: f64,
    /// Gaussian sampling algorithm's target number of samples to take for each color.
    #[arg(
        short,
        long,
        default_value_t = 512,
        conflicts_with = "shape",
        conflicts_with = "power",
        conflicts_with = "nearest",
        conflicts_with = "preserve"
    )]
    iterations: usize,
}

#[derive(Default, Clone, Debug, ValueEnum)]
enum Algorithm {
    /// Shepard's method (RBF interpolation using the inverse distance function).
    /// Params: --power, --nearest, --lum
    ShepardsMethod,
    /// Radial Basis Function interpolation using the Gaussian function.
    /// Params: --shape, --nearest, --lum
    #[default]
    GaussianRBF,
    /// Radial Basis Function interpolation using a linear function.
    /// Params: --nearest, --lum
    LinearRBF,
    /// Optimized version of the original ImageMagick approach which applies gaussian noise
    /// to each color and averages nearest neighbors together.
    /// Params: --mean, --std_dev, --iterations, --lum
    GaussianSampling,
    /// Simple, non-interpolated, nearest neighbor alorithm.
    /// Params: --lum
    NearestNeighbor,
}

impl LutArgs {
    fn generate(&self) -> Image {
        let name = self.name();
        let mut sp = Spinner::with_timer_and_stream(
            Spinners::Dots3,
            format!("Generating \"{name}\" LUT..."),
            spinners::Stream::Stderr,
        );
        let time = Instant::now();

        let lut = match self.algorithm {
            Algorithm::ShepardsMethod => ShepardRemapper::new(
                &self.collect(),
                self.power,
                self.nearest,
                self.luminosity,
                self.preserve,
            )
            .generate_lut(self.level),
            Algorithm::GaussianRBF => GaussianRemapper::new(
                &self.collect(),
                self.shape,
                self.nearest,
                self.luminosity,
                self.preserve,
            )
            .generate_lut(self.level),
            Algorithm::LinearRBF => LinearRemapper::new(
                &self.collect(),
                self.nearest,
                self.luminosity,
                self.preserve,
            )
            .generate_lut(self.level),
            Algorithm::GaussianSampling => GaussianSamplingRemapper::new(
                &self.collect(),
                self.mean,
                self.std_dev,
                self.iterations,
                self.luminosity,
                SEED,
            )
            .generate_lut(self.level),
            Algorithm::NearestNeighbor => {
                NearestNeighborRemapper::new(&self.collect(), self.luminosity)
                    .generate_lut(self.level)
            },
        };

        sp.stop_and_persist("✔", format!("Generated {name} LUT in {:?}", time.elapsed()));

        lut
    }

    fn collect(&self) -> Vec<[u8; 3]> {
        let mut colors = parse_hex(&self.custom_colors);
        if let Some(palette) = self.palette {
            colors.append(&mut palette.get().to_vec());
        }
        colors
    }

    fn name(&self) -> String {
        let mut name = String::new();

        if let Some(palette) = self.palette {
            let p_name = palette.to_possible_value().unwrap();
            if !self.custom_colors.is_empty() {
                name.push_str("custom-");
            }
            name.push_str(p_name.get_name());
        } else {
            name.push_str("custom");
        };

        name
    }

    fn detail_string(&self) -> String {
        let mut buf = format!(
            "hald{}_{:?}_lum{}",
            self.level, self.algorithm, self.luminosity
        );
        match self.algorithm {
            Algorithm::GaussianSampling => buf.push_str(&format!(
                "_{}_{}_{}",
                self.mean, self.std_dev, self.iterations
            )),
            Algorithm::ShepardsMethod => {
                buf.push_str(&format!("_pow{}_near{}", self.power, self.nearest));
            },
            Algorithm::GaussianRBF => {
                buf.push_str(&format!("_shape{}_near{}", self.shape, self.nearest));
            },
            Algorithm::LinearRBF => {
                buf.push_str(&format!("_near{}", self.nearest));
            },
            Algorithm::NearestNeighbor => {},
        }
        buf
    }
}

fn main() {
    let total_time = Instant::now();

    let BinArgs { subcommand } = BinArgs::parse();

    match subcommand {
        // Generate and save a hald clut identity
        Subcommands::Generate { lut_args, output } => {
            let colors = lut_args.collect();
            if colors.is_empty() {
                min_colors_error()
            }

            save_image(
                output.unwrap_or(PathBuf::from(format!(
                    "{}_{}.png",
                    lut_args.name(),
                    lut_args.detail_string(),
                ))),
                &lut_args.generate(),
            );

            println!("Finished in {:?}", total_time.elapsed());
        },
        // Correct an image using a hald clut identity
        Subcommands::Apply {
            output,
            lut_args,
            hald_clut,
            images,
            cache,
            force,
        } => {
            let colors = lut_args.collect();
            // load or generate the lut
            let hald_clut = {
                match hald_clut {
                    Some(path) => load_image(path),
                    None => {
                        if cache {
                            let path = cache_dir().unwrap_or(".cache/".into()).join("lutgen");
                            if !path.exists() {
                                std::fs::create_dir_all(&path)
                                    .expect("failed to create cache directory");
                            }

                            let cache_name =
                                format!("{}_{}", lut_args.name(), lut_args.detail_string());
                            let path = path.join(cache_name).with_extension("png");
                            if path.exists() && !force {
                                load_image(path)
                            } else {
                                if colors.is_empty() {
                                    min_colors_error()
                                }
                                let lut = lut_args.generate();
                                cache_image(path, &lut);
                                lut
                            }
                        } else {
                            if colors.is_empty() {
                                min_colors_error()
                            }
                            lut_args.generate()
                        }
                    },
                }
            };

            for image_path in &images {
                let mut image_buf = load_image(image_path);

                let mut sp = Spinner::with_stream(
                    Spinners::Dots3,
                    format!("Applying LUT to {image_path:?}..."),
                    spinners::Stream::Stderr,
                );
                let time = Instant::now();
                identity::correct_image(&mut image_buf, &hald_clut);
                sp.stop_and_persist(
                    "✔",
                    format!("Applied LUT to {image_path:?} in {:?}", time.elapsed()),
                );

                let path = if images.len() > 1 {
                    // For multiple images, the output path is always treated as a directory
                    let path = output.clone().unwrap_or(PathBuf::from(lut_args.name()));
                    if !path.exists() {
                        create_dir_all(&path).expect("failed to create output directory");
                    }
                    path.join(image_path.file_name().unwrap())
                } else {
                    // For single images
                    match &output {
                        // If user provided a path
                        Some(path) => {
                            // Create the parent directory if needed
                            if let Some(parent) = path.parent() {
                                if !path.exists() {
                                    create_dir_all(parent)
                                        .expect("failed to create output directory");
                                }
                            }

                            if path.is_dir() {
                                path.join(image_path.file_name().unwrap())
                            } else {
                                path.clone()
                            }
                        },
                        // No path, so save the file under a palette name directory
                        None => {
                            let path = PathBuf::from(lut_args.name());
                            if !path.exists() {
                                create_dir_all(&path).expect("failed to create output directory");
                            }
                            path.join(image_path.file_name().unwrap())
                        },
                    }
                };

                save_image(path, &image_buf);
            }

            println!("Finished in {:?}", total_time.elapsed());
        },
        Subcommands::Patch {
            files,
            write,
            lut_args,
        } => {
            let time = Instant::now();
            let lut = lut_args.generate();
            let len = files.len();
            let mut buffer = vec![];

            let re = Regex::new(REGEX).expect("failed to build regex");
            for file in files {
                let time = Instant::now();
                let contents = std::fs::read_to_string(&file).expect("not a text file");

                let counter = &mut 0u32;
                let replaced = re.replace_all(&contents, |caps: &Captures| {
                    *counter += 1;
                    if caps.get(1).is_some() {
                        let rgb = parse_one_hex(&caps[2]);
                        let [r, g, b] = correct_pixel(&rgb, &lut, lut_args.level);
                        format!("#{r:02x}{g:02x}{b:02x}")
                    } else if caps.get(3).is_some() {
                        let inner: Vec<u8> = caps[4]
                            .split(',')
                            .map(|s| s.trim().parse().expect("invalid rgb code"))
                            .collect();
                        let [r, g, b] =
                            correct_pixel(&[inner[0], inner[1], inner[2]], &lut, lut_args.level);
                        format!("rgb({r}, {g}, {b})")
                    } else if caps.get(5).is_some() {
                        let inner: Vec<u8> = caps[6]
                            .split(',')
                            .map(|s| s.trim().parse().expect("invalid rgb point"))
                            .collect();
                        let [r, g, b] =
                            correct_pixel(&[inner[0], inner[1], inner[2]], &lut, lut_args.level);
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
                    }

                    // Compute diff for the file
                    let input = imara_diff::intern::InternedInput::new(
                        contents.as_str(),
                        replaced.as_str(),
                    );
                    let diff = imara_diff::diff(
                        imara_diff::Algorithm::Histogram,
                        &input,
                        imara_diff::UnifiedDiffBuilder::new(&input),
                    );
                    buffer.push(format!(
                        "--- a/{file}\n+++ b/{file}\n{diff}",
                        file = file.to_string_lossy()
                    ));
                }
            }

            eprintln!(
                "Finished generating patch for {len} file{} in {:?}\n",
                if len > 1 { "s" } else { "" },
                time.elapsed()
            );

            // Print the patches
            print!("{}", buffer.join("\n"));
            stdout().flush().expect("failed to print diff");
        },
        Subcommands::Palette { palette, name_only } => {
            // Print a palette
            fn print(palette: &Palette) {
                eprintln!(
                    "\x1b[4m{}\x1b[0m",
                    palette.name().trim_start_matches('_').replace('_', "-")
                );
                for &color in palette.get() {
                    let [r, g, b] = color;
                    let Oklab { l, .. } = srgb_to_oklab(color.into());
                    let fg = if l < 0.5 {
                        "\x1b[38;2;255;255;255m"
                    } else {
                        "\x1b[38;2;0;0;0m"
                    };
                    println!("\x1b[48;2;{r};{g};{b}m{fg}#{r:02x}{g:02x}{b:02x}\x1b[0m");
                }
            }

            if let Some(palette) = palette {
                print(&palette)
            } else {
                for palette in Palette::value_variants() {
                    if name_only {
                        println!(
                            "{}",
                            palette.name().trim_start_matches('_').replace('_', "-")
                        )
                    } else {
                        eprintln!();
                        print(palette);
                    }
                }
            }
        },
        Subcommands::Completions { shell } => {
            // Generate the completions and exit immediately
            let mut cmd = BinArgs::command();
            let name = cmd.get_name().to_string();
            eprintln!("Generating completions for {shell}");
            generate(shell, &mut cmd, name, &mut std::io::stdout());
        },
    };
}

fn parse_one_hex(code: &str) -> [u8; 3] {
    // parse hex string into rgb
    let mut hex = (*code).trim_start_matches('#').to_string();

    match hex.len() {
        3 => {
            // Extend 3 character hex colors
            hex = hex.chars().flat_map(|a| [a, a]).collect();
        },
        6 => {},
        _ => {
            parse_hex_error(code);
            exit(2);
        },
    }

    if let Ok(channel_bytes) = u32::from_str_radix(&hex, 16) {
        let r = ((channel_bytes >> 16) & 0xFF) as u8;
        let g = ((channel_bytes >> 8) & 0xFF) as u8;
        let b = (channel_bytes & 0xFF) as u8;
        [r, g, b]
    } else {
        parse_hex_error(code);
        exit(2);
    }
}

fn parse_hex(codes: &[String]) -> Vec<[u8; 3]> {
    codes.iter().map(|code| parse_one_hex(code)).collect()
}

fn load_image<P: AsRef<Path>>(path: P) -> Image {
    let path = path.as_ref();
    let time = Instant::now();
    let mut sp = Spinner::with_stream(
        Spinners::Dots3,
        format!("Loading {path:?}..."),
        spinners::Stream::Stderr,
    );

    let lut = Reader::open(path)
        .expect("failed to open image")
        .with_guessed_format()
        .expect("failed to guess image format")
        .decode()
        .expect("failed to decode image")
        .to_rgb8();

    sp.stop_and_persist("✔", format!("Loaded {path:?} in {:?}", time.elapsed()));

    lut
}

fn save_image<P: AsRef<Path>>(path: P, image: &Image) {
    let path = path.as_ref();
    let time = Instant::now();
    let mut sp = Spinner::with_stream(
        Spinners::Dots3,
        format!("Saving output to {path:?}..."),
        spinners::Stream::Stderr,
    );

    image.save(path).expect("failed to save image");

    sp.stop_and_persist(
        "✔",
        format!("Saved output to {path:?} in {:?}", time.elapsed()),
    );
}

fn cache_image<P: AsRef<Path>>(path: P, image: &Image) {
    let path = path.as_ref();
    let time = Instant::now();
    let mut sp = Spinner::with_stream(
        Spinners::Dots3,
        format!("Caching {path:?}..."),
        spinners::Stream::Stderr,
    );

    image.save(path).expect("failed to save cache image");

    sp.stop_and_persist("✔", format!("Cached {path:?} in {:?}", time.elapsed()));
}

fn min_colors_error() {
    let mut err = clap::Error::new(ErrorKind::InvalidValue).with_cmd(&BinArgs::command());
    err.insert(
        ContextKind::InvalidArg,
        ContextValue::String("COLORS".into()),
    );
    err.insert(ContextKind::InvalidValue, ContextValue::String("".into()));
    err.insert(
        ContextKind::ValidValue,
        ContextValue::Strings(vec![
            "-p PALETTE".to_string(),
            "#123456".to_string(),
            "#abcdef".to_string(),
            "ABC123".to_string(),
        ]),
    );
    err.print().unwrap();
    exit(2);
}

fn parse_hex_error(input: &str) {
    let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(&BinArgs::command());
    err.insert(
        ContextKind::InvalidArg,
        ContextValue::String("hex color".into()),
    );
    err.insert(
        ContextKind::InvalidValue,
        ContextValue::String(input.to_string()),
    );
    err.print().unwrap();
    exit(2);
}
