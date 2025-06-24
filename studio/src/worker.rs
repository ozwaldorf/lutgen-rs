use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use egui::Context;
use lutgen::GenerateLut;
use uuid::Uuid;

use crate::state::{Common, CommonRbf, GaussianRbfArgs, GaussianSamplingArgs, ShepardsMethodArgs};

pub enum FrontendEvent {
    PickFile,
    LoadFile(PathBuf),
    Apply(Vec<[u8; 3]>, Common, LutAlgorithmArgs),
}

pub enum BackendEvent {
    Error(String),
    PickFile(PathBuf),
    Applied(f64),
    SetImage(Vec<u8>, u32, u32, String),
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

impl Display for BackendEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendEvent::Error(e) => f.write_str(&format!("Error: {e}")),
            BackendEvent::PickFile(path_buf) => f.write_str(&format!("Loaded {path_buf:?}")),
            BackendEvent::Applied(palette) => {
                f.write_str(&format!("Applied palette \"{palette}\" to image"))
            },
            BackendEvent::SetImage(_, _, _, _) => Ok(()),
        }
    }
}

pub struct WorkerHandle {
    tx: Sender<FrontendEvent>,
    rx: Receiver<BackendEvent>,
}

impl WorkerHandle {
    pub fn pick_file(&self) {
        self.tx
            .send(FrontendEvent::PickFile)
            .expect("failed to send to worker");
    }

    pub fn load_file(&self, path: &Path) {
        self.tx
            .send(FrontendEvent::LoadFile(path.to_path_buf()))
            .expect("failed to send load file to worker");
    }

    pub fn apply_palette(&self, palette: Vec<[u8; 3]>, common: Common, args: LutAlgorithmArgs) {
        self.tx
            .send(FrontendEvent::Apply(palette, common, args))
            .expect("failed to send apply request to worker");
    }

    pub fn poll_event(&self) -> Option<BackendEvent> {
        self.rx.try_recv().ok()
    }
}

pub struct Worker {
    tx: Sender<BackendEvent>,
    current_image: Option<lutgen::RgbaImage>,
}

impl Worker {
    pub fn spawn(ctx: Context) -> WorkerHandle {
        let (tx, worker_rx) = channel();
        let (worker_tx, rx) = channel();

        std::thread::spawn(move || {
            let mut worker = Worker {
                tx: worker_tx,
                current_image: None,
            };
            while let Ok(event) = worker_rx.recv() {
                let res = match event {
                    FrontendEvent::PickFile => worker.pick_file(),
                    FrontendEvent::LoadFile(path) => worker.load_file(&path),
                    FrontendEvent::Apply(palette, common, args) => {
                        worker.apply_palette(palette, common, args)
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

        WorkerHandle { tx, rx }
    }

    fn send_set_image(&self, image: Vec<u8>, width: u32, height: u32, path: Option<PathBuf>) {
        self.tx
            .send(BackendEvent::SetImage(
                image,
                width,
                height,
                format!(
                    "bytes://{}",
                    path.map(|p| p.display().to_string())
                        .unwrap_or(Uuid::new_v4().to_string())
                ),
            ))
            .expect("failed to send image to ui thread")
    }

    /// Open a file dialog and load the image into the window
    fn pick_file(&mut self) -> Result<(), String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
            .pick_file()
        {
            self.load_file(&path)?;
            self.tx
                .send(BackendEvent::PickFile(path))
                .map_err(|_| "failed to send file path to ui thread".to_string())?;
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
            image.to_vec(),
            image.height(),
            image.width(),
            Some(path.to_path_buf()),
        );
        self.current_image = Some(image);
        Ok(())
    }

    /// Apply a palette to the currently loaded image
    fn apply_palette(
        &self,
        palette: Vec<[u8; 3]>,
        common: Common,
        args: LutAlgorithmArgs,
    ) -> Result<(), String> {
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
                .generate_lut(common.level)
            },
            LutAlgorithmArgs::ShepardsMethod { rbf, args } => {
                lutgen::interpolation::ShepardRemapper::new(
                    &palette,
                    *args.power,
                    rbf.nearest,
                    *common.lum_factor,
                    rbf.preserve,
                )
                .generate_lut(common.level)
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
                .generate_lut(common.level)
            },
            LutAlgorithmArgs::NearestNeighbor => {
                lutgen::interpolation::NearestNeighborRemapper::new(&palette, *common.lum_factor)
                    .generate_lut(common.level)
            },
        };

        // remap image
        lutgen::identity::correct_image(&mut image, &lut);

        // encode edited image
        // TODO: figure out how the hell to load raw bytes
        let mut buf = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("failed to encode image: {e}"))?;

        self.send_set_image(image.to_vec(), image.height(), image.width(), None);
        self.tx
            .send(BackendEvent::Applied(0.))
            .expect("failed to send applied event to ui thread");

        println!("applied palette");
        Ok(())
    }
}
