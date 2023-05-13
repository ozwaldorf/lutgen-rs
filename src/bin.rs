use std::{path::PathBuf, time::Instant};

use clap::{arg, command, Parser, ValueEnum};
use lutgen::{generate_v0_lut, generate_v1_lut};

mod palette;

const SEED: u64 = u64::from_be_bytes(*b"42080085");

/// Generate Hald CLUT images from arbitrary colors using gaussian distribution.
///
/// The default mean are equivelant to imagemagick's
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Algorithm to generate the LUT with.
    #[arg(short, value_enum, default_value = "v1")]
    algorithm: Algorithm,
    /// Path to write the generated file to. Defaults to the current dir with some parameters (ex: `./hald_clut_v1_4_20_512.png`)
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
        algorithm,
        level,
        output,
        mean,
        std_dev,
        iterations,
    } = Args::parse();

    println!("Generating {algorithm:?} LUT... (level: {level}, mean: {mean}, std_dev: {std_dev}, iterations: {iterations})");

    let now = Instant::now();
    let palette_lut = match algorithm {
        Algorithm::V0 => generate_v0_lut(
            &palette::catppuccin::MOCHA,
            level,
            mean,
            std_dev,
            iterations,
            SEED,
        ),
        Algorithm::V1 => generate_v1_lut(
            &palette::catppuccin::MOCHA,
            level,
            mean,
            std_dev,
            iterations,
            SEED,
        ),
    };

    let filename = output.unwrap_or(PathBuf::from(format!(
        "hald_clut_{level}_{algorithm:?}_{mean}_{std_dev}_{iterations}.png"
    )));

    println!("Finished in {:?}\nSaving to {filename:?}", now.elapsed());

    palette_lut.save(filename).unwrap();
}
