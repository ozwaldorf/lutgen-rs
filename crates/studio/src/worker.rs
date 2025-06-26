use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::Context;
use image::ColorType;
use lutgen::GenerateLut;

use crate::state::{Common, CommonRbf, GaussianRbfArgs, GaussianSamplingArgs, ShepardsMethodArgs};

pub enum FrontendEvent {
    PickFile,
    LoadFile(PathBuf),
    Apply(Vec<[u8; 3]>, Common, LutAlgorithmArgs, Arc<AtomicBool>),
    SaveAs,
}

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
    PickFile(PathBuf, Duration),
    Applied(Duration),
    SetImage {
        path: Option<PathBuf>,
        image: Arc<[u8]>,
        dim: (u32, u32),
    },
}

impl Display for BackendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendEvent::Error(e) => f.write_str(&format!("Error: {e}")),
            BackendEvent::PickFile(path_buf, time) => {
                f.write_str(&format!("Loaded {path_buf:?} in {time:.2?}"))
            },
            BackendEvent::Applied(time) => f.write_str(&format!("Corrected image in {time:.2?}")),
            BackendEvent::SetImage { .. } => Ok(()),
        }
    }
}

pub struct WorkerHandle {
    tx: Sender<FrontendEvent>,
    rx: Receiver<BackendEvent>,
    abort: Arc<AtomicBool>,
}

impl WorkerHandle {
    pub fn pick_file(&self) {
        self.tx
            .send(FrontendEvent::PickFile)
            .expect("failed to send to worker");
    }

    pub fn save_as(&self) {
        self.tx
            .send(FrontendEvent::SaveAs)
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
    last_render: Arc<[u8]>,
}

impl Worker {
    pub fn spawn(ctx: Context) -> WorkerHandle {
        let (tx, worker_rx) = channel();
        let (worker_tx, rx) = channel();
        let abort = Arc::new(AtomicBool::new(false));

        std::thread::spawn(move || {
            let mut worker = Worker {
                tx: worker_tx,
                current_image: None,
                last_render: Default::default(),
            };
            while let Ok(event) = worker_rx.recv() {
                let res = match event {
                    FrontendEvent::PickFile => worker.pick_file(),
                    FrontendEvent::SaveAs => worker.save_as(),
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

    fn send_set_image(&mut self, path: Option<PathBuf>, image: Vec<u8>, dim: (u32, u32)) {
        self.last_render = image.into();
        self.tx
            .send(BackendEvent::SetImage {
                image: self.last_render.clone(),
                path,
                dim,
            })
            .expect("failed to send image to ui thread")
    }

    /// Open a file dialog and load the image into the window
    fn pick_file(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
            .pick_file()
        {
            let time = Instant::now();
            self.load_file(&path)?;
            self.tx
                .send(BackendEvent::PickFile(path, time.elapsed()))
                .map_err(|_| "failed to send file path to ui thread".to_string())?;
        }

        Ok(())
    }

    fn save_as(&self) -> Result<(), String> {
        if let Some(image) = &self.current_image
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("image", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
                .save_file()
        {
            image::save_buffer(
                path,
                &self.last_render,
                image.width(),
                image.height(),
                ColorType::Rgba8,
            )
            .map_err(|e| format!("failed to encode image: {e}"))?
        }

        Ok(())
    }

    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();

        let mut buf = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("failed to encode image: {e}"))?;

        self.send_set_image(
            Some(path.to_path_buf()),
            image.to_vec(),
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

        // get image or return
        let Some(mut image) = self.current_image.clone() else {
            // do nothing if no image is loaded
            return Ok(());
        };

        // generate lut from arguments
        let lut = match args {
            LutAlgorithmArgs::GaussianRbf { rbf, args } => {
                lutgen::interpolation::GaussianRemapper::new(
                    &palette,
                    *args.shape,
                    rbf.nearest,
                    *common.lum_factor,
                    rbf.preserve,
                )
                .generate_lut_with_interrupt(common.level, abort)
            },
            LutAlgorithmArgs::ShepardsMethod { rbf, args } => {
                lutgen::interpolation::ShepardRemapper::new(
                    &palette,
                    *args.power,
                    rbf.nearest,
                    *common.lum_factor,
                    rbf.preserve,
                )
                .generate_lut_with_interrupt(common.level, abort)
            },
            LutAlgorithmArgs::GaussianSampling { args } => {
                lutgen::interpolation::GaussianSamplingRemapper::new(
                    &palette,
                    *args.mean,
                    *args.std_dev,
                    args.iterations,
                    *common.lum_factor,
                    args.seed,
                )
                .generate_lut_with_interrupt(common.level, abort)
            },
            LutAlgorithmArgs::NearestNeighbor => {
                lutgen::interpolation::NearestNeighborRemapper::new(&palette, *common.lum_factor)
                    .generate_lut_with_interrupt(common.level, abort)
            },
        }
        .ok_or("aborted".to_string())?;

        // remap image
        lutgen::identity::correct_image(&mut image, &lut);

        // encode edited image
        // TODO: figure out how the hell to load raw bytes
        let mut buf = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("failed to encode image: {e}"))?;

        self.send_set_image(None, image.to_vec(), (image.height(), image.width()));
        self.tx
            .send(BackendEvent::Applied(time.elapsed()))
            .expect("failed to send applied event to ui thread");

        Ok(())
    }
}
