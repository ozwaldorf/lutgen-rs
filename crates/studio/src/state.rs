use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;

use egui::TextureHandle;
use log::{error, info};

use crate::palette::DynamicPalette;
use crate::updates::UpdateInfo;
use crate::utils::Hashed;
use crate::worker::{BackendEvent, ImageSource};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct UiState {
    // main window state
    pub show_about: bool,
    #[serde(skip)]
    pub processing: bool,
    #[cfg_attr(target_arch = "wasm32", serde(skip))]
    pub current_image: Option<PathBuf>,
    #[serde(skip)]
    pub image_texture: Option<TextureHandle>,
    #[serde(skip)]
    pub edited_texture: Option<TextureHandle>,
    #[serde(skip)]
    pub show_original: bool,
    #[serde(skip)]
    pub update: Option<UpdateInfo>,
    #[serde(skip)]
    pub last_event: String,

    // side panel state
    pub palette_selection: DynamicPalette,
    pub palette: Vec<[u8; 3]>,
    pub current_alg: LutAlgorithm,
    pub guassian_rbf: GaussianRbfArgs,
    pub shepards_method: ShepardsMethodArgs,
    pub guassian_sampling: GaussianSamplingArgs,
    pub common_rbf: CommonRbf,
    pub common: Common,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            // default is true for first starts
            show_about: true,
            processing: false,
            update: None,
            last_event: "Started.".to_string(),
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
        }
    }
}

impl UiState {
    /// Handle incoming backend events from the worker
    pub fn handle_event(&mut self, ctx: &egui::Context, event: BackendEvent) {
        self.last_event = event.to_string();
        info!("Received event: {}", self.last_event);

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

                // hide spinner
                self.processing = false;

                match source {
                    ImageSource::Image(path) => {
                        // for newly opened images from file picker
                        self.current_image = Some(path);
                        self.image_texture = Some(texture);
                        self.edited_texture = None;
                        self.show_original = true;
                    },
                    ImageSource::Edited(_) => {
                        // for edited output
                        self.edited_texture = Some(texture);
                        self.show_original = false;
                    },
                }
            },
            BackendEvent::Update(update) => {
                self.update = Some(update);
            },
            #[cfg(target_arch = "wasm32")]
            BackendEvent::SaveData(_, data) => {
                use web_sys::wasm_bindgen::JsCast;

                self.processing = false;
                let filename = self
                    .current_image
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or("lutgen.png".to_string());

                // create a download link
                let win = web_sys::window().expect("failed to get window");
                let doc = win.document().expect("failed to get document");
                let link = doc.create_element("a").expect("failed to create link");
                link.set_attribute("href", &data)
                    .expect("failed to set data");
                link.set_attribute("download", &filename)
                    .expect("failed to set download name");

                // click it
                let link: web_sys::HtmlAnchorElement =
                    web_sys::HtmlAnchorElement::unchecked_from_js(link.into());
                link.click();

                // cleanup
                link.remove();
            },
        }
    }

    /// Collect the lutgen cli arguments used to replicate the current parameters
    pub fn cli_args(&self) -> Vec<String> {
        macro_rules! arg {
            ($args:expr, $flag:expr, $arg:expr, $default:expr) => {
                if $arg != $default {
                    $args.push($flag.to_string());
                    $args.push($arg.to_string());
                }
            };
            ($args:expr, $flag:expr, bool $arg:expr, $default:expr) => {
                if $arg != $default {
                    $args.push($flag.to_string());
                }
            };
        }

        let mut args = Vec::new();

        // builtin palettes
        if let DynamicPalette::Builtin(palette) = self.palette_selection {
            arg!(args, "-p", palette.to_string(), "");
        }

        // common rbf args
        match self.current_alg {
            LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                arg!(args, "-n", self.common_rbf.nearest, 16);
            },
            _ => {},
        }

        // common args
        arg!(args, "-l", self.common.level, 12);
        arg!(args, "-L", self.common.lum_factor.0, 0.7);
        arg!(args, "-P", bool self.common.preserve, false);

        // algorithm specific args
        match self.current_alg {
            LutAlgorithm::GaussianRbf => {
                arg!(args, "-s", self.guassian_rbf.shape.0, 128.);
            },
            LutAlgorithm::ShepardsMethod => {
                arg!(args, "-p", self.shepards_method.power.0, 4.);
            },
            LutAlgorithm::GaussianSampling => {
                arg!(args, "-m", self.guassian_sampling.mean.0, 0.);
                arg!(args, "-s", self.guassian_sampling.std_dev.0, 20.);
                arg!(args, "-i", self.guassian_sampling.iterations, 512);
                arg!(args, "-S", self.guassian_sampling.seed, 42080085);
            },
            LutAlgorithm::NearestNeighbor => {},
        }

        // image path
        if let Some(path) = &self.current_image {
            args.push(path.display().to_string());
        }

        // finally custom palette colors
        if matches!(self.palette_selection, DynamicPalette::Custom(_)) {
            args.push("--".to_string());
            for [r, g, b] in &self.palette {
                args.push(format!("{r:0x}{g:0x}{b:0x}"));
            }
        }

        args
    }

    /// Reset the current arguments based on the selected algorithm
    pub fn reset_current_args(&mut self) {
        let default = Self::default();
        self.common = default.common;
        match self.current_alg {
            LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                self.common_rbf = default.common_rbf;
            },
            _ => {},
        }
        match self.current_alg {
            LutAlgorithm::GaussianRbf => {
                self.guassian_rbf = default.guassian_rbf;
            },
            LutAlgorithm::ShepardsMethod => {
                self.shepards_method = default.shepards_method;
            },
            LutAlgorithm::GaussianSampling => {
                self.guassian_sampling = default.guassian_sampling;
            },
            LutAlgorithm::NearestNeighbor => {},
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
    /// Preserve the original image's luminocity values after interpolation.
    pub preserve: bool,
    /// Factor to multiply luminocity values by. Effectively weights the interpolation to prefer
    /// more colorful or more greyscale/unsaturated matches. Usually paired with `--preserve`.
    pub lum_factor: Hashed<f64>,
    /// Hald clut level to generate. A level of 16 stores a value for the entire sRGB color space.
    pub level: u8,
}

impl Default for Common {
    fn default() -> Self {
        Self {
            preserve: true,
            lum_factor: Hashed(0.7),
            #[cfg(not(target_arch = "wasm32"))]
            level: 12,
            #[cfg(target_arch = "wasm32")]
            level: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, serde::Deserialize, serde::Serialize)]
pub struct CommonRbf {
    /// Number of nearest colors to consider when interpolating. 0 uses all available colors.
    pub nearest: usize,
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
