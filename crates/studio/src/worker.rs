use std::fmt::Display;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::Context;
use image::ColorType;
use log::{debug, info};
use lutgen::GenerateLut;

use crate::color::Color;
use crate::state::{Common, CommonRbf, GaussianRbfArgs, GaussianSamplingArgs, ShepardsMethodArgs};
use crate::updates::{check_for_updates, UpdateInfo};

pub enum FrontendEvent {
    LoadFile(PathBuf),
    Apply(Vec<[u8; 3]>, Common, LutAlgorithmArgs, Arc<AtomicBool>),
    SaveAs(PathBuf),
}

#[derive(Hash, Debug)]
pub enum LutAlgorithmArgs {
    GaussianRbf {
        rbf: CommonRbf,
        args: GaussianRbfArgs,
    },
    ShepardsMethod {
        rbf: CommonRbf,
        args: ShepardsMethodArgs,
    },
    GaussianSampling {
        args: GaussianSamplingArgs,
    },
    NearestNeighbor,
}

pub enum BackendEvent {
    Error(String),
    Update(UpdateInfo),
    SetImage {
        time: Duration,
        source: ImageSource,
        image: Arc<[u8]>,
        dim: (u32, u32),
    },
}

impl Display for BackendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendEvent::Error(e) => format!("Error: {e}").fmt(f),
            BackendEvent::Update(_) => Ok(()),
            BackendEvent::SetImage {
                time,
                dim: (x, y),
                source: path,
                ..
            } => match path {
                ImageSource::Image(_) => format!("Opened {x}x{y} image in {time:.2?}").fmt(f),
                ImageSource::Edited(_) => {
                    format!("Generated and applied LUT to image in {time:.2?}").fmt(f)
                },
            },
        }
    }
}

pub enum ImageSource {
    Image(PathBuf),
    Edited(u64),
}

impl Display for ImageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageSource::Image(path_buf) => Display::fmt(&path_buf.display(), f),
            ImageSource::Edited(hash) => Display::fmt(hash, f),
        }
    }
}

pub struct WorkerHandle {
    tx: Sender<FrontendEvent>,
    rx: Receiver<BackendEvent>,
    abort: Arc<AtomicBool>,
}

impl WorkerHandle {
    pub fn new(ctx: egui::Context) -> Self {
        Worker::spawn(ctx)
    }

    pub fn save_as(&self, path: PathBuf) {
        self.tx
            .send(FrontendEvent::SaveAs(path))
            .expect("failed to send save as request to worker");
    }

    pub fn load_file(&self, path: &Path) {
        self.tx
            .send(FrontendEvent::LoadFile(path.to_path_buf()))
            .expect("failed to send load file to worker");
    }

    pub fn apply_palette(&mut self, palette: Vec<[u8; 3]>, common: Common, args: LutAlgorithmArgs) {
        // cancel previous run and init a new abort signal
        self.abort.store(true, std::sync::atomic::Ordering::Relaxed);
        self.abort = Arc::new(AtomicBool::new(false));

        self.tx
            .send(FrontendEvent::Apply(
                palette,
                common,
                args,
                self.abort.clone(),
            ))
            .expect("failed to send apply request to worker");
    }

    pub fn poll_event(&self) -> Option<BackendEvent> {
        self.rx.try_recv().ok()
    }
}

pub struct Worker {
    tx: Sender<BackendEvent>,
    current_image: Option<lutgen::RgbaImage>,
    hasher: DefaultHasher,
    last_render: Arc<[u8]>,
}

impl Worker {
    fn spawn(ctx: Context) -> WorkerHandle {
        let (tx, worker_rx) = channel();
        let (worker_tx, rx) = channel();
        let abort = Arc::new(AtomicBool::new(false));

        // Spawn thread to fetch the latest version and send it to the frontend if newer
        let worker_tx_clone = worker_tx.clone();
        std::thread::spawn(move || {
            if let Ok(Some(update)) = check_for_updates() {
                worker_tx_clone
                    .send(BackendEvent::Update(update))
                    .expect("failed to send update info to frontend");
            }
        });

        std::thread::spawn(move || {
            let mut worker = Worker {
                tx: worker_tx,
                current_image: None,
                hasher: DefaultHasher::new(),
                last_render: Default::default(),
            };
            while let Ok(event) = worker_rx.recv() {
                let res = match event {
                    FrontendEvent::SaveAs(path) => worker.save_as(path),
                    FrontendEvent::LoadFile(path) => worker.load_file(&path),
                    FrontendEvent::Apply(palette, common, args, abort) => {
                        worker.apply_palette(palette, common, args, abort)
                    },
                };
                if let Err(e) = res {
                    worker
                        .tx
                        .send(BackendEvent::Error(e))
                        .expect("failed to send backend error to ui thread");
                }
                ctx.request_repaint();
            }
        });

        WorkerHandle { tx, rx, abort }
    }

    fn send_set_image(
        &mut self,
        time: Duration,
        source: ImageSource,
        image: Arc<[u8]>,
        dim: (u32, u32),
    ) {
        self.tx
            .send(BackendEvent::SetImage {
                time,
                source,
                image,
                dim,
            })
            .expect("failed to send image to ui thread")
    }

    fn save_as(&self, path: PathBuf) -> Result<(), String> {
        if self.last_render.is_empty() {
            return Err("Image must be applied at least once".into());
        }
        if let Some(image) = &self.current_image {
            image::save_buffer(
                path,
                &self.last_render,
                image.width(),
                image.height(),
                ColorType::Rgba8,
            )
            .map_err(|e| format!("failed to encode image: {e}"))?;
        }

        Ok(())
    }

    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let time = Instant::now();
        let image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();

        // hash image
        self.hasher = DefaultHasher::new();
        image.hash(&mut self.hasher);

        self.send_set_image(
            time.elapsed(),
            ImageSource::Image(path.to_path_buf()),
            image.to_vec().into(),
            (image.height(), image.width()),
        );
        self.current_image = Some(image);
        Ok(())
    }

    /// Apply a palette to the currently loaded image
    fn apply_palette(
        &mut self,
        palette: Vec<[u8; 3]>,
        common: Common,
        args: LutAlgorithmArgs,
        abort: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let time = Instant::now();

        let Some(mut image) = self.current_image.clone() else {
            // do nothing if no image is loaded
            return Ok(());
        };

        // hash arguments with existing image hash
        let mut hasher = self.hasher.clone();
        palette.hash(&mut hasher);
        common.hash(&mut hasher);
        args.hash(&mut hasher);
        let hash = hasher.finish();

        // generate lut from arguments
        info!("Generating LUT with args:\n{common:?}\n{args:?}");
        debug!(
            "LUT input palette ({} colors):\n{}",
            palette.len(),
            palette
                .chunks(5)
                .map(|v| v
                    .iter()
                    .cloned()
                    .map(|v| Color(v).to_string())
                    .collect::<Vec<_>>()
                    .join(", "))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let lut = match args {
            LutAlgorithmArgs::GaussianRbf { rbf, args } => {
                lutgen::interpolation::GaussianRemapper::new(
                    &palette,
                    *args.shape,
                    rbf.nearest,
                    *common.lum_factor,
                    common.preserve,
                )
                .par_generate_lut_with_interrupt(common.level, abort)
            },
            LutAlgorithmArgs::ShepardsMethod { rbf, args } => {
                lutgen::interpolation::ShepardRemapper::new(
                    &palette,
                    *args.power,
                    rbf.nearest,
                    *common.lum_factor,
                    common.preserve,
                )
                .par_generate_lut_with_interrupt(common.level, abort)
            },
            LutAlgorithmArgs::GaussianSampling { args } => {
                lutgen::interpolation::GaussianSamplingRemapper::new(
                    &palette,
                    *args.mean,
                    *args.std_dev,
                    args.iterations,
                    *common.lum_factor,
                    args.seed,
                    common.preserve,
                )
                .par_generate_lut_with_interrupt(common.level, abort)
            },
            LutAlgorithmArgs::NearestNeighbor => {
                lutgen::interpolation::NearestNeighborRemapper::new(
                    &palette,
                    *common.lum_factor,
                    common.preserve,
                )
                .par_generate_lut_with_interrupt(common.level, abort)
            },
        }
        .ok_or("aborted".to_string())?;

        // remap image
        lutgen::identity::correct_image_with_level(&mut image, &lut, common.level);

        self.last_render = image.to_vec().into();
        self.send_set_image(
            time.elapsed(),
            ImageSource::Edited(hash),
            self.last_render.clone(),
            (image.height(), image.width()),
        );

        Ok(())
    }
}
