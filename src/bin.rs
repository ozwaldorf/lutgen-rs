use std::{
    path::{Path, PathBuf},
    process::exit,
    time::Instant,
};

use clap::{
    arg, command,
    error::{ContextKind, ContextValue, ErrorKind},
    Args, CommandFactory, Parser, Subcommand, ValueEnum,
};
use clap_complete::{generate, Shell};
use dirs::cache_dir;
use exoquant::SimpleColorSpace;
use lutgen::{identity, interpolation::*, GenerateLut, Image};
use lutgen_palettes::Palette;
use spinners::{Spinner, Spinners};

const SEED: u64 = u64::from_be_bytes(*b"42080085");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct BinArgs {
    /// Path to write output to.
    #[arg(short, long)]
    #[clap(global = true)]
    output: Option<PathBuf>,
    #[command(subcommand)]
    subcommand: Option<Subcommands>,
    #[command(flatten)]
    lutargs: LutArgs,
    #[arg(long, value_name = "SHELL")]
    completions: Option<Shell>,
}

#[derive(Args, Debug)]
struct LutArgs {
    /// Custom hexidecimal colors to add to the palette.
    /// If `-p` is not used to specify a base palette, at least 1 color is required.
    #[clap(global = true)]
    custom_colors: Vec<String>,
    /// Predefined popular color palettes. Use `lutgen -p` to view all options. Compatible with
    /// custom colors.
    #[arg(short, long, value_enum, hide_possible_values = true)]
    #[clap(global = true)]
    palette: Option<Palette>,
    /// Hald level (ex: 8 = 512x512 image)
    #[arg(short, long, default_value_t = 8)]
    #[clap(global = true)]
    level: u8,
    /// Algorithm to remap the LUT with.
    #[arg(short, long, value_enum, default_value = "gaussian-rbf")]
    #[clap(global = true)]
    algorithm: Algorithm,
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
    #[clap(global = true)]
    nearest: usize,
    /// Gaussian RBF's shape parameter.
    /// Higher means less gradient between colors, lower mean more.
    #[arg(
        short,
        long,
        default_value_t = 96.0,
        conflicts_with = "mean",
        conflicts_with = "std_dev",
        conflicts_with = "iterations"
    )]
    #[clap(global = true)]
    shape: f64,
    /// Shepard algorithm's power parameter.
    #[arg(
        long,
        default_value_t = 4.0,
        conflicts_with = "mean",
        conflicts_with = "std_dev",
        conflicts_with = "iterations"
    )]
    #[clap(global = true)]
    power: f64,
    /// Gaussian sampling algorithm's mean parameter.
    #[arg(short, long, default_value_t = 0.0)]
    #[clap(global = true)]
    mean: f64,
    /// Gaussian sampling algorithm's standard deviation parameter.
    #[arg(long, default_value_t = 20.0)]
    #[clap(global = true)]
    std_dev: f64,
    /// Gaussian sampling algorithm's target number of samples to take for each color.
    #[arg(short, long, default_value_t = 512)]
    #[clap(global = true)]
    iterations: usize,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Correct an image using a hald clut, either generating it, or loading it externally.
    Apply {
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
            conflicts_with = "force"
        )]
        hald_clut: Option<PathBuf>,
        /// Enable caching the generated LUT
        #[arg(short, long, default_value_t = false)]
        cache: bool,
        /// Force overwriting the cached LUT.
        #[arg(short, long, default_value_t = false, requires = "cache")]
        force: bool,
        /// Image to correct with a hald clut.
        image: PathBuf,
    },
}

#[derive(Default, Clone, Debug, ValueEnum)]
enum Algorithm {
    /// Shepard's method (RBF interpolation using the inverse distance function).
    /// Params: --power, --nearest
    ShepardsMethod,
    /// Radial Basis Function interpolation using the Gaussian function.
    /// Params: --shape, --nearest
    GaussianRBF,
    /// Radial Basis Function interpolation using a linear function.
    /// Params: --nearest
    LinearRBF,
    /// Optimized version of the original ImageMagick approach which applies gaussian noise
    /// to each color and averages nearest neighbors together.
    /// Params: --mean, --std_dev, --iterations
    #[default]
    GaussianSampling,
    /// Simple, non-interpolated, nearest neighbor alorithm.
    NearestNeighbor,
}

impl LutArgs {
    fn generate(&self) -> Image {
        let colorspace = SimpleColorSpace::default();
        let name = self.name();
        let mut sp = Spinner::new(Spinners::Dots3, format!("Generating \"{name}\" LUT..."));
        let time = Instant::now();

        let lut = match self.algorithm {
            Algorithm::ShepardsMethod => {
                ShepardRemapper::new(&self.collect(), self.power, self.nearest)
                    .generate_lut(self.level)
            },
            Algorithm::GaussianRBF => {
                GaussianRemapper::new(&self.collect(), self.shape, self.nearest)
                    .generate_lut(self.level)
            },
            Algorithm::LinearRBF => {
                LinearRemapper::new(&self.collect(), self.nearest).generate_lut(self.level)
            },
            Algorithm::GaussianSampling => GaussianSamplingRemapper::new(
                &self.collect(),
                self.mean,
                self.std_dev,
                self.iterations,
                SEED,
                colorspace,
            )
            .generate_lut(self.level),
            Algorithm::NearestNeighbor => {
                NearestNeighborRemapper::new(&self.collect(), colorspace).generate_lut(self.level)
            },
        };

        sp.stop_and_persist(
            "✔",
            format!("Generated \"{name}\" LUT in {:?}", time.elapsed()),
        );

        lut
    }

    fn collect(&self) -> Vec<[u8; 3]> {
        let mut colors = self
            .custom_colors
            .iter()
            .map(|s| {
                // parse hex string into rgb
                let hex = (*s).trim_start_matches('#');
                if hex.len() != 6 {
                    parse_hex_error(s);
                    exit(2);
                }
                if let Ok(channel_bytes) = u32::from_str_radix(hex, 16) {
                    let r = ((channel_bytes >> 16) & 0xFF) as u8;
                    let g = ((channel_bytes >> 8) & 0xFF) as u8;
                    let b = (channel_bytes & 0xFF) as u8;
                    [r, g, b]
                } else {
                    parse_hex_error(s);
                    exit(2);
                }
            })
            .collect::<Vec<_>>();

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
        let mut buf = format!("{}_{:?}", self.level, self.algorithm);
        match self.algorithm {
            Algorithm::GaussianSampling => buf.push_str(&format!(
                "_{}_{}_{}",
                self.mean, self.std_dev, self.iterations
            )),
            Algorithm::ShepardsMethod => {
                buf.push_str(&format!("_{}_{}", self.power, self.nearest));
            },
            Algorithm::GaussianRBF => {
                buf.push_str(&format!("_{}_{}", self.shape, self.nearest));
            },
            Algorithm::LinearRBF => {
                buf.push_str(&format!("_{}", self.nearest));
            },
            Algorithm::NearestNeighbor => {},
        }
        buf
    }
}

fn main() {
    let total_time = Instant::now();

    let BinArgs {
        subcommand,
        output,
        lutargs,
        completions,
    } = BinArgs::parse();

    if let Some(shell) = completions {
        // Generate the completions and exit immediately
        let mut cmd = BinArgs::command();
        let name = cmd.get_name().to_string();
        eprintln!("Generating completions for {shell}");
        generate(shell, &mut cmd, name, &mut std::io::stdout());
        std::process::exit(0);
    }

    let colors = lutargs.collect();

    match subcommand {
        // Generate and save a hald clut identity
        None => {
            if colors.is_empty() {
                min_colors_error()
            }

            save_image(
                output.unwrap_or(PathBuf::from(format!(
                    "{}_hald_clut_{}.png",
                    lutargs.name(),
                    lutargs.detail_string(),
                ))),
                &lutargs.generate(),
            );
        },
        // Correct an image using a hald clut identity
        Some(Subcommands::Apply {
            hald_clut,
            image,
            cache,
            force,
        }) => {
            // load or generate the lut
            let (hald_clut, details) = {
                match hald_clut {
                    Some(path) => (load_image(path), "custom".into()),
                    None => {
                        let cache_name = format!("{}_{}", lutargs.name(), lutargs.detail_string());

                        if cache {
                            let path = cache_dir().unwrap_or(".cache/".into()).join("lutgen");
                            if !path.exists() {
                                std::fs::create_dir_all(&path)
                                    .expect("failed to create cache directory");
                            }

                            let path = path.join(&cache_name).with_extension("png");
                            if path.exists() && !force {
                                (load_image(path), cache_name)
                            } else {
                                if colors.is_empty() {
                                    min_colors_error()
                                }
                                let lut = lutargs.generate();
                                cache_image(path, &lut);
                                (lut, cache_name)
                            }
                        } else {
                            if colors.is_empty() {
                                min_colors_error()
                            }
                            (lutargs.generate(), cache_name)
                        }
                    },
                }
            };

            let mut image_buf = load_image(&image);

            let mut sp = Spinner::new(Spinners::Dots3, format!("Applying LUT to {image:?}..."));
            let time = Instant::now();
            identity::correct_image(&mut image_buf, &hald_clut);
            sp.stop_and_persist(
                "✔",
                format!("Applied LUT to {image:?} in {:?}", time.elapsed()),
            );

            save_image(
                output.unwrap_or(PathBuf::from(format!(
                    "{}_{details}.png",
                    image.with_extension("").display()
                ))),
                &image_buf,
            )
        },
    };

    println!("Finished in {:?}", total_time.elapsed());
}

fn load_image<P: AsRef<Path>>(path: P) -> Image {
    let path = path.as_ref();
    let mut sp = Spinner::new(Spinners::Dots3, format!("Loading {path:?}..."));
    let time = Instant::now();
    let lut = image::open(path).expect("failed to open image").to_rgb8();
    sp.stop_and_persist("✔", format!("Loaded {path:?} in {:?}", time.elapsed()));
    lut
}

fn save_image<P: AsRef<Path>>(path: P, image: &Image) {
    let path = path.as_ref();
    let mut sp = Spinner::new(Spinners::Dots3, format!("Saving output to {path:?}..."));
    let time = Instant::now();
    image.save(path).expect("failed to save image");
    sp.stop_and_persist(
        "✔",
        format!("Saved output to {path:?} in {:?}", time.elapsed()),
    );
}

fn cache_image<P: AsRef<Path>>(path: P, image: &Image) {
    let path = path.as_ref();
    let mut sp = Spinner::new(Spinners::Dots3, format!("Caching {path:?}..."));
    let time = Instant::now();
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
