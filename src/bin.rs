use std::{path::PathBuf, process::exit, time::Instant};

use clap::{
    arg, command,
    error::{ContextKind, ContextValue, ErrorKind},
    CommandFactory, Parser, ValueEnum,
};
use exoquant::Color;
use lutgen::{generate_v0_lut, generate_v1_lut};
use lutgen_palettes::Palette;

const SEED: u64 = u64::from_be_bytes(*b"42080085");

/// Generate Hald CLUT images from arbitrary colors using gaussian distribution.
///
/// The default mean are equivelant to imagemagick's
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of custom colors to add to the palette.
    /// If `-p` is not used to specify a base palette, at least 1 color is required.
    custom_colors: Vec<String>,
    /// Add colors from a predefined base palette. Use `lutgen -p` to view all options.
    #[arg(short, value_enum, hide_possible_values = true)]
    palette: Option<Palette>,
    /// Algorithm to generate the LUT with.
    #[arg(short, value_enum, default_value = "v1")]
    algorithm: Algorithm,
    /// Path to write the generated file to.
    /// Defaults to the current dir with some parameters (ex: `./hald_clut_v1_4_20_512.png`)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// HaldCLUT color depth. 8 bit = 512x512 image
    #[arg(short, long, default_value_t = 8)]
    level: u32,
    /// Mean for the gaussian distribution.
    ///
    /// The default value is equivelant to `-attenuate 1` in imagemagick.
    #[arg(short, long, default_value_t = 4.0)]
    mean: f64,
    /// Standard deviation for the gaussian distribution.
    ///
    /// The default value is equivelant to `-attenuate 1` in imagemagick.
    #[arg(short, long, default_value_t = 20.0)]
    std_dev: f64,
    /// Number of iterations to average together.
    ///
    /// Equivelant to `-duplicate 512` in imagemagick.
    #[arg(short, long, default_value_t = 512)]
    iterations: usize,
}

#[derive(Default, Clone, Debug, ValueEnum)]
enum Algorithm {
    #[default]
    V1,
    V0,
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
            // parse hex string into rgb
            let hex = s.trim_start_matches('#');
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
            Color::new(r, g, b, 255)
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
        err.insert(ContextKind::Usage, ContextValue::String("asdf".into()));

        err.print().unwrap();

        exit(2);
    }

    println!("Generating {algorithm:?} LUT... (palette: {name}, level: {level}, mean: {mean}, std_dev: {std_dev}, iterations: {iterations})");

    let now = Instant::now();

    let palette_lut = match algorithm {
        Algorithm::V0 => generate_v0_lut(&colors, level, mean, std_dev, iterations, SEED),
        Algorithm::V1 => generate_v1_lut(&colors, level, mean, std_dev, iterations, SEED),
    };

    let filename = output.unwrap_or(PathBuf::from(format!(
        "{name}_hald_clut_{level}_{algorithm:?}_{mean}_{std_dev}_{iterations}.png",
    )));

    println!("Finished in {:?}\nSaving to {filename:?}", now.elapsed());

    palette_lut.save(filename).unwrap();
}
