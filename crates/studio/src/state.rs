use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;

use egui::TextureHandle;
use log::{debug, error};

use crate::palette::DynamicPalette;
use crate::utils::Hashed;
use crate::worker::{BackendEvent, ImageSource};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct UiState {
    // main window state
    pub current_image: Option<PathBuf>,
    #[serde(skip)]
    pub image_texture: Option<TextureHandle>,
    #[serde(skip)]
    pub edited_texture: Option<TextureHandle>,
    #[serde(skip)]
    pub show_original: bool,

    // side panel state
    pub palette_selection: DynamicPalette,
    pub palette: Vec<[u8; 3]>,
    pub current_alg: LutAlgorithm,
    pub guassian_rbf: GaussianRbfArgs,
    pub shepards_method: ShepardsMethodArgs,
    pub guassian_sampling: GaussianSamplingArgs,
    pub common_rbf: CommonRbf,
    pub common: Common,

    // about dialog
    pub show_about: bool,

    #[serde(skip)]
    pub last_event: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_image: None,
            image_texture: None,
            edited_texture: None,
            show_original: false,

            palette_selection: DynamicPalette::Builtin(lutgen_palettes::Palette::Carburetor),
            palette: lutgen_palettes::Palette::Carburetor.get().to_vec(),
            current_alg: Default::default(),
            guassian_rbf: Default::default(),
            shepards_method: Default::default(),
            guassian_sampling: Default::default(),
            common_rbf: Default::default(),
            common: Default::default(),

            // default is true for first starts
            show_about: true,

            last_event: "Started.".to_string(),
        }
    }
}

impl UiState {
    /// Handle incoming backend events from the worker
    pub fn handle_event(&mut self, ctx: &egui::Context, event: BackendEvent) {
        self.last_event = event.to_string();
        if !self.last_event.is_empty() {
            debug!("{}", self.last_event);
        }

        match event {
            BackendEvent::Error(e) => {
                error!("{e}");
            },
            BackendEvent::SetImage {
                source,
                image,
                dim: (width, height),
                ..
            } => {
                // load image into a new egui texture
                let texture = ctx.load_texture(
                    format!("bytes://{source}",),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [height as usize, width as usize],
                        &image,
                    ),
                    egui::TextureOptions::NEAREST
                        .with_mipmap_mode(Some(egui::TextureFilter::Nearest)),
                );

                match source {
                    ImageSource::Image(path) => {
                        // for newly opened images from file picker
                        self.current_image = Some(path);
                        self.image_texture = Some(texture);
                        self.show_original = true;
                    },
                    ImageSource::Edited(_) => {
                        // for edited output
                        self.edited_texture = Some(texture);
                        self.show_original = false;
                    },
                }
            },
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::VariantArray,
)]
pub enum LutAlgorithm {
    #[default]
    GaussianRbf,
    ShepardsMethod,
    GaussianSampling,
    NearestNeighbor,
}

#[derive(Clone, Copy, Debug, Hash, serde::Deserialize, serde::Serialize)]
pub struct Common {
    /// Factor to multiply luminocity values by. Effectively weights the interpolation to prefer
    /// more colorful or more greyscale/unsaturated matches. Usually paired with `--preserve`.
    pub lum_factor: Hashed<f64>,
    /// Hald clut level to generate. A level of 16 stores a value for the entire sRGB color space.
    pub level: u8,
}

impl Default for Common {
    fn default() -> Self {
        Self {
            lum_factor: Hashed(0.7),
            level: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, serde::Deserialize, serde::Serialize)]
pub struct CommonRbf {
    /// Number of nearest colors to consider when interpolating. 0 uses all available colors.
    pub nearest: usize,
    /// Preserve the original image's luminocity values after interpolation.
    pub preserve: bool,
}

impl Default for CommonRbf {
    fn default() -> Self {
        Self {
            nearest: 16,
            preserve: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, serde::Deserialize, serde::Serialize)]
pub struct GaussianRbfArgs {
    /// Shape parameter for the default Gaussian RBF interpolation. Effectively creates more or
    /// less blending between colors in the palette, where bigger numbers equal less blending.
    /// Effect is heavily dependant on the number of nearest colors used.
    pub shape: Hashed<f64>,
}

impl Default for GaussianRbfArgs {
    fn default() -> Self {
        Self {
            shape: Hashed(128.),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, serde::Deserialize, serde::Serialize)]
pub struct ShepardsMethodArgs {
    /// Power parameter for shepard's method.
    pub power: Hashed<f64>,
}

impl Default for ShepardsMethodArgs {
    fn default() -> Self {
        Self { power: Hashed(4.) }
    }
}

#[derive(Clone, Copy, Debug, Hash, serde::Deserialize, serde::Serialize)]
pub struct GaussianSamplingArgs {
    /// Average amount of noise to apply in each iteration.
    pub mean: Hashed<f64>,
    /// Standard deviation parameter for the noise applied in each iteration.
    pub std_dev: Hashed<f64>,
    /// Number of iterations of noise to apply to each pixel.
    pub iterations: usize,
    /// Seed for noise rng.
    pub seed: u64,
}

impl Default for GaussianSamplingArgs {
    fn default() -> Self {
        Self {
            mean: Hashed(4.),
            std_dev: Hashed(20.),
            iterations: 128,
            seed: 42080085,
        }
    }
}
