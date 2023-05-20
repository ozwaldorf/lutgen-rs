use std::{path::PathBuf, process::exit, time::Instant};

use clap::{
    arg, command,
    error::{ContextKind, ContextValue, ErrorKind},
    CommandFactory, Parser, ValueEnum,
};
use exoquant::SimpleColorSpace;
use lutgen::{generate_lut, interpolated_remap::*, Image, Palette};

const SEED: u64 = u64::from_be_bytes(*b"42080085");

/// Generate Hald CLUT images from arbitrary colors using gaussian distribution.
///
/// The default mean are equivelant to imagemagick's
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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
    level: u32,
    /// Mean for the gaussian distribution.
    #[arg(short, long, default_value_t = 4.0)]
    mean: f64,
    /// Standard deviation for the gaussian distribution.
    #[arg(short, long, default_value_t = 20.0)]
    std_dev: f64,
    /// Number of iterations to average together.
    #[arg(short, long, default_value_t = 512)]
    iterations: usize,
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

impl Algorithm {
    fn generate(
        &self,
        palette: &[[u8; 3]],
        level: u32,
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
    let Args {
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
                let r = ((channel_bytes >> 16) & 0xFF) as u8;
                let g = ((channel_bytes >> 8) & 0xFF) as u8;
                let b = (channel_bytes & 0xFF) as u8;
                [r, g, b]
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

    if colors.is_empty() {
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

    println!("Generating {algorithm:?} LUT... (palette: {name}, level: {level}, mean: {mean}, std_dev: {std_dev}, iterations: {iterations})");

    let time = Instant::now();
    let palette_lut = algorithm.generate(&colors, level, mean, std_dev, iterations, SEED);
    let time = time.elapsed();

    // Save output
    let filename = output.unwrap_or(PathBuf::from(format!(
        "{name}_hald_clut_{level}_{algorithm:?}_{mean}_{std_dev}_{iterations}.png",
    )));

    println!("Finished in {time:?}\nSaving to {filename:?}");
    palette_lut.save(filename).unwrap();
}
