use std::{path::PathBuf, process::exit, time::Instant};

use clap::{
    arg, command,
    error::{ContextKind, ContextValue, ErrorKind},
    CommandFactory, Parser, Subcommand, ValueEnum,
};
use exoquant::{Color, SimpleColorSpace};
use lutgen::{generate_lut, identity, interpolated_remap::*, Image, Palette};
use spinners::{Spinner, Spinners};

const SEED: u64 = u64::from_be_bytes(*b"42080085");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    subcommand: Option<Subcommands>,

    /// List of custom hexidecimal colors to add to the palette.
    /// If `-p` is not used to specify a base palette, at least 1 color is required.
    custom_colors: Vec<String>,
    /// Add colors from a predefined base palette. Use `lutgen -p` to view all options.
    #[arg(short, value_enum, hide_possible_values = true)]
    palette: Option<Palette>,
    /// Interpolated remapping algorithm to generate the LUT with.
    #[arg(short, value_enum, default_value = "gaussian-v1")]
    algorithm: Algorithm,
    /// Path to write the generated file to.
    /// Defaults to the current dir with some parameters (ex: `./hald_clut_v1_4_20_512.png`)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Hald level (ex: 8 = 512x512 image)
    #[arg(short, long, default_value_t = 8)]
    level: u8,
    /// Mean for the gaussian distribution.
    #[arg(short, long, default_value_t = 0.0)]
    mean: f64,
    /// Standard deviation for the gaussian distribution.
    #[arg(short, long, default_value_t = 20.0)]
    std_dev: f64,
    /// Number of gaussian samples to average together.
    #[arg(short, long, default_value_t = 512)]
    iterations: usize,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Correct an image using a hald clut, either provided or generated on the fly.
    Correct {
        /// Optionally use an external hald-clut. If unspecified, the arguments provided
        /// will be used to generate the lut on the fly.
        #[arg(long)]
        hald_clut: Option<PathBuf>,
        /// Image to correct with a hald clut.
        image: PathBuf,
    },
}

/// Generate Hald CLUT images from arbitrary colors using gaussian distribution.

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

impl Algorithm {
    fn generate(
        &self,
        palette: &[Color],
        level: u8,
        mean: f64,
        std_dev: f64,
        iterations: usize,
        seed: u64,
    ) -> Image {
        let colorspace = SimpleColorSpace::default();
        match self {
            Self::GaussianV0 => generate_lut::<GaussianV0Remapper<_>>(
                level,
                palette,
                GaussianV0Params {
                    mean,
                    std_dev,
                    iterations,
                    seed,
                    colorspace,
                },
            ),
            Self::GaussianV1 => generate_lut::<GaussianV1Remapper<_>>(
                level,
                palette,
                GaussianV1Params {
                    mean,
                    std_dev,
                    iterations,
                    seed,
                    colorspace,
                },
            ),
            Self::NearestNeighbor => {
                generate_lut::<NearestNeighborRemapper<_>>(level, palette, colorspace)
            }
        }
    }
}

fn main() {
    let total_time = Instant::now();

    let Args {
        subcommand,
        custom_colors: custom_palette,
        palette,
        algorithm,
        level,
        output,
        mean,
        std_dev,
        iterations,
    } = Args::parse();

    let mut colors = custom_palette
        .iter()
        .map(|s| {
            fn show_hex_err(input: &str) {
                let mut err =
                    clap::Error::new(ErrorKind::ValueValidation).with_cmd(&Args::command());
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

            // parse hex string into rgb
            let hex = s.trim_start_matches('#');
            if hex.len() != 6 {
                show_hex_err(s);
                exit(2);
            }
            if let Ok(channel_bytes) = u32::from_str_radix(hex, 16) {
                let r = (channel_bytes >> 16) & 0xFF;
                let g = (channel_bytes >> 8) & 0xFF;
                let b = channel_bytes & 0xFF;
                Color::new(r as u8, g as u8, b as u8, 255)
            } else {
                show_hex_err(s);
                exit(2);
            }
        })
        .collect::<Vec<_>>();

    let mut name = String::new();
    if let Some(palette) = palette {
        let p_name = palette.to_possible_value().unwrap();
        if colors.is_empty() {
            name.push_str(p_name.get_name());
        } else {
            name.push_str("custom-");
            name.push_str(p_name.get_name());
        }
        colors.append(&mut palette.get().to_vec());
    } else {
        name.push_str("custom");
    };

    let (output, filename) = match subcommand {
        None => {
            // Generate and save a hald clut identity
            if colors.is_empty() {
                min_colors_error()
            }

            let mut sp = Spinner::new(Spinners::Dots3, format!("Generating `{name}` LUT..."));
            let time = Instant::now();

            let palette_lut = algorithm.generate(&colors, level, mean, std_dev, iterations, SEED);

            sp.stop_and_persist(
                "✔",
                format!("Generated `{name}` LUT in {:?}", time.elapsed()),
            );

            (
                palette_lut,
                output.unwrap_or(PathBuf::from(format!(
                    "{name}_hald_clut_{level}_{algorithm:?}_{mean}_{std_dev}_{iterations}.png",
                ))),
            )
        }
        Some(Subcommands::Correct { hald_clut, image }) => {
            // Correct an image using a hald clut identity

            // load or generate the lut
            let (hald_clut, details) = match hald_clut {
                Some(path) => {
                    let mut sp = Spinner::new(Spinners::Dots3, format!("Loading {path:?}..."));
                    let time = Instant::now();
                    let lut = image::open(&path).unwrap().to_rgb8();
                    sp.stop_and_persist("✔", format!("Loaded {path:?} in {:?}", time.elapsed()));
                    (lut, "custom".into())
                }
                None => {
                    if colors.is_empty() {
                        min_colors_error()
                    }

                    let mut sp =
                        Spinner::new(Spinners::Dots3, format!("Generating `{name}` LUT..."));
                    let time = Instant::now();

                    let palette_lut =
                        algorithm.generate(&colors, level, mean, std_dev, iterations, SEED);

                    sp.stop_and_persist(
                        "✔",
                        format!("Generated `{name}` LUT in {:?}", time.elapsed()),
                    );

                    (
                        palette_lut,
                        format!("{name}_{level}_{algorithm:?}_{mean}_{std_dev}_{iterations}"),
                    )
                }
            };

            // apply the lut to the image
            let mut sp = Spinner::new(Spinners::Dots3, format!("Applying LUT to {image:?}..."));
            let time = Instant::now();

            let mut image_buf = image::open(&image).unwrap().to_rgb8();
            identity::correct_image(&mut image_buf, &hald_clut);

            sp.stop_and_persist(
                "✔",
                format!("Applied LUT to {image:?} in {:?}", time.elapsed()),
            );

            (
                image_buf,
                output.unwrap_or(PathBuf::from(format!(
                    "{}_{details}.png",
                    image.with_extension("").display()
                ))),
            )
        }
    };

    let mut sp = Spinner::new(Spinners::Dots3, format!("Saving output to {filename:?}..."));
    let time = Instant::now();

    output.save(&filename).unwrap();

    sp.stop_and_persist(
        "✔",
        format!("Saved output to {filename:?} in {:?}", time.elapsed()),
    );

    println!("Finished in {:?}", total_time.elapsed());
}

fn min_colors_error() {
    let mut err = clap::Error::new(ErrorKind::TooFewValues).with_cmd(&Args::command());
    err.insert(
        ContextKind::InvalidArg,
        ContextValue::String("COLORS".into()),
    );
    err.insert(ContextKind::ActualNumValues, ContextValue::Number(0));
    err.insert(ContextKind::MinValues, ContextValue::Number(1));
    err.print().unwrap();
    exit(2);
}
