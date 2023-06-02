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
use dirs::cache_dir;
use exoquant::SimpleColorSpace;
use lutgen::{generate_lut, identity, interpolated_remap::*, Image, Palette};
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
}

#[derive(Args, Debug)]
struct LutArgs {
    /// Custom hexidecimal colors to add to the palette.
    /// If `-p` is not used to specify a base palette, at least 1 color is required.
    #[clap(global = true)]
    custom_colors: Vec<String>,
    /// Predefined popular color palettes. Use `lutgen -p` to view all options. Compatible with custom colors.
    #[arg(short, value_enum, hide_possible_values = true)]
    #[clap(global = true)]
    palette: Option<Palette>,
    /// Remapping algorithm to generate the LUT with.
    #[arg(short, value_enum, default_value = "gaussian-v1")]
    #[clap(global = true)]
    algorithm: Algorithm,
    /// Hald level (ex: 8 = 512x512 image)
    #[arg(short, long, default_value_t = 8)]
    #[clap(global = true)]
    level: u8,
    /// Mean for gaussian distribution.
    #[arg(short, long, default_value_t = 0.0)]
    #[clap(global = true)]
    mean: f64,
    /// Standard deviation for gaussian distribution.
    #[arg(short, long, default_value_t = 20.0)]
    #[clap(global = true)]
    std_dev: f64,
    /// Number of gaussian samples for each color to average together.
    #[arg(short, long, default_value_t = 512)]
    #[clap(global = true)]
    iterations: usize,
}

impl LutArgs {
    fn generate(&self, seed: u64) -> Image {
        let colorspace = SimpleColorSpace::default();
        let name = self.name();
        let mut sp = Spinner::new(Spinners::Dots3, format!("Generating \"{name}\" LUT..."));
        let time = Instant::now();

        let lut = match self.algorithm {
            Algorithm::GaussianV0 => generate_lut::<GaussianV0Remapper<_>>(
                self.level,
                &self.collect(),
                GaussianV0Params {
                    mean: self.mean,
                    std_dev: self.std_dev,
                    iterations: self.iterations,
                    seed,
                    colorspace,
                },
            ),
            Algorithm::GaussianV1 => generate_lut::<GaussianV1Remapper<_>>(
                self.level,
                &self.collect(),
                GaussianV1Params {
                    mean: self.mean,
                    std_dev: self.std_dev,
                    iterations: self.iterations,
                    seed,
                    colorspace,
                },
            ),
            Algorithm::NearestNeighbor => {
                generate_lut::<NearestNeighborRemapper<_>>(self.level, &self.collect(), colorspace)
            }
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
        format!(
            "{}_{:?}_{}_{}_{}",
            self.level, self.algorithm, self.mean, self.std_dev, self.iterations
        )
    }
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Correct an image using a hald clut, either generating it, or loading it externally.
    Apply {
        /// An external hald-clut to use. Conflicts with all lut generation related arguments.
        #[arg(
            long,
            conflicts_with = "lutargs",
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
    /// Fastest algorithm for gaussian interpolated remapping
    #[default]
    GaussianV1,
    /// Original algorithm for gaussian interpolated remapping
    GaussianV0,
    /// Non-interpolated algorithm that remaps to the nearest neighbor
    NearestNeighbor,
}

impl Algorithm {}

fn main() {
    let total_time = Instant::now();

    let BinArgs {
        subcommand,
        output,
        lutargs,
    } = BinArgs::parse();

    let colors = lutargs.collect();

    match subcommand {
        None => {
            // Generate and save a hald clut identity
            if colors.is_empty() {
                min_colors_error()
            }

            save_image(
                output.unwrap_or(PathBuf::from(format!(
                    "{}_hald_clut_{}.png",
                    lutargs.name(),
                    lutargs.detail_string(),
                ))),
                &lutargs.generate(SEED),
            );
        }
        Some(Subcommands::Apply {
            hald_clut,
            image,
            cache,
            force,
        }) => {
            // Correct an image using a hald clut identity

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
                                let lut = lutargs.generate(SEED);
                                cache_image(path, &lut);
                                (lut, cache_name)
                            }
                        } else {
                            if colors.is_empty() {
                                min_colors_error()
                            }
                            (lutargs.generate(SEED), cache_name)
                        }
                    }
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
        }
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
    let mut err = clap::Error::new(ErrorKind::TooFewValues).with_cmd(&BinArgs::command());
    err.insert(
        ContextKind::InvalidArg,
        ContextValue::String("COLORS".into()),
    );
    err.insert(ContextKind::ActualNumValues, ContextValue::Number(0));
    err.insert(ContextKind::MinValues, ContextValue::Number(1));
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
