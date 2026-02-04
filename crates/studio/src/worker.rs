use std::fmt::Display;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::Arc;

use log::{debug, info};
use lutgen::GenerateLut;
use web_time::{Duration, Instant};

use crate::color::Color;
use crate::state::{
    BlurArgs,
    Common,
    CommonRbf,
    GaussianRbfArgs,
    GaussianSamplingArgs,
    ShepardsMethodArgs,
};
use crate::updates::UpdateInfo;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum FrontendEvent {
    LoadFile(PathBuf, #[cfg(target_arch = "wasm32")] Vec<u8>),
    Apply(Vec<[u8; 3]>, Common, LutAlgorithmArgs, Arc<AtomicBool>),
    SaveAs(
        #[cfg(not(target_arch = "wasm32"))] PathBuf,
        #[cfg(target_arch = "wasm32")] image::ImageFormat,
    ),
}

#[derive(serde::Serialize, serde::Deserialize, Hash, Debug)]
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
    GaussianBlur {
        args: BlurArgs,
    },
    NearestNeighbor,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum BackendEvent {
    Error(String),
    Update(UpdateInfo),
    SetImage {
        time: Duration,
        source: ImageSource,
        image: Arc<[u8]>,
        dim: (u32, u32),
    },
    #[cfg(target_arch = "wasm32")]
    SaveData(Duration, String, image::ImageFormat),
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
            #[cfg(target_arch = "wasm32")]
            BackendEvent::SaveData(time, _, format) => {
                format!("Encoded {format:?} for download in {time:.2?}").fmt(f)
            },
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
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
    #[cfg(target_arch = "wasm32")]
    bridge: gloo_worker::WorkerBridge<Worker>,

    #[cfg(not(target_arch = "wasm32"))]
    tx: std::sync::mpsc::Sender<FrontendEvent>,

    rx: std::sync::mpsc::Receiver<BackendEvent>,
    abort: Arc<AtomicBool>,
}

impl WorkerHandle {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn(ctx: egui::Context) -> Self {
        let (tx, worker_rx) = channel();
        let (worker_tx, rx) = channel();
        let abort = Arc::new(AtomicBool::new(false));

        // Spawn thread to fetch the latest version and send it to the frontend if newer
        let worker_tx_cloned = worker_tx.clone();
        std::thread::spawn(move || {
            if let Ok(Some(update)) = crate::updates::check_for_updates() {
                worker_tx_cloned
                    .send(BackendEvent::Update(update))
                    .expect("failed to send update info to frontend");
            }
        });

        std::thread::spawn(move || {
            let mut worker = Worker {
                current_image: None,
                hasher: DefaultHasher::new(),
                last_render: Default::default(),
            };
            while let Ok(event) = worker_rx.recv() {
                if let Some(event) = worker.handle_event(event) {
                    worker_tx
                        .send(event)
                        .expect("failed to send backend event to ui thread");
                }
                ctx.request_repaint();
            }
        });

        WorkerHandle { tx, rx, abort }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(ctx: egui::Context) -> Self {
        use gloo_worker::Spawnable;

        let abort = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let bridge = Worker::spawner()
            .callback(move |event| {
                tx.send(event)
                    .expect("failed to send backend event to worker handle");
                ctx.request_repaint();
            })
            .spawn("worker.js");

        Self { rx, bridge, abort }
    }

    fn send(&self, event: FrontendEvent) {
        #[cfg(not(target_arch = "wasm32"))]
        self.tx
            .send(event)
            .expect("failed to send save as request to worker");
        #[cfg(target_arch = "wasm32")]
        self.bridge.send(event);
    }

    pub fn save_as(
        &self,
        #[cfg(not(target_arch = "wasm32"))] item: PathBuf,
        #[cfg(target_arch = "wasm32")] item: image::ImageFormat,
    ) {
        self.send(FrontendEvent::SaveAs(item));
    }

    pub fn load_file(&self, path: PathBuf, #[cfg(target_arch = "wasm32")] bytes: Vec<u8>) {
        #[cfg(not(target_arch = "wasm32"))]
        self.send(FrontendEvent::LoadFile(path));
        #[cfg(target_arch = "wasm32")]
        self.send(FrontendEvent::LoadFile(path, bytes));
    }

    pub fn apply_palette(&mut self, palette: Vec<[u8; 3]>, common: Common, args: LutAlgorithmArgs) {
        // cancel previous run and init a new abort signal
        self.abort.store(true, std::sync::atomic::Ordering::Relaxed);
        self.abort = Arc::new(AtomicBool::new(false));

        self.send(FrontendEvent::Apply(
            palette,
            common,
            args,
            self.abort.clone(),
        ))
    }

    pub fn poll_event(&self) -> Option<BackendEvent> {
        self.rx.try_recv().ok()
    }
}

pub struct Worker {
    current_image: Option<lutgen::RgbaImage>,
    hasher: DefaultHasher,
    last_render: Arc<[u8]>,
}

impl Worker {
    fn handle_event(&mut self, event: FrontendEvent) -> Option<BackendEvent> {
        let res = match event {
            #[cfg(not(target_arch = "wasm32"))]
            FrontendEvent::SaveAs(path) => self.save_as(path),
            #[cfg(target_arch = "wasm32")]
            FrontendEvent::SaveAs(format) => self.save_as(format),
            #[cfg(not(target_arch = "wasm32"))]
            FrontendEvent::LoadFile(path) => self.load_file(&path),
            #[cfg(target_arch = "wasm32")]
            FrontendEvent::LoadFile(path, bytes) => self.load_file(&path, bytes),
            FrontendEvent::Apply(palette, common, args, abort) => {
                self.apply_palette(palette, common, args, abort)
            },
        };
        match res {
            Ok(event) => event,
            Err(e) => Some(BackendEvent::Error(e)),
        }
    }

    fn save_as(
        &self,
        #[cfg(not(target_arch = "wasm32"))] path: PathBuf,
        #[cfg(target_arch = "wasm32")] format: image::ImageFormat,
    ) -> Result<Option<BackendEvent>, String> {
        if self.last_render.is_empty() {
            return Err("Image must be applied at least once".into());
        }
        if let Some(image) = &self.current_image {
            #[cfg(not(target_arch = "wasm32"))]
            if image::save_buffer(
                &path,
                &self.last_render,
                image.width(),
                image.height(),
                image::ColorType::Rgba8,
            )
            .is_err()
            {
                // image format likely doesn't support transparency, convert to RGB
                let buffer: Vec<u8> = self
                    .last_render
                    .chunks_exact(4)
                    .flat_map(|v| &v[0..3])
                    .cloned()
                    .collect();
                image::save_buffer(
                    path,
                    &buffer,
                    image.width(),
                    image.height(),
                    image::ColorType::Rgb8,
                )
                .map_err(|e| format!("failed to encode image: {e}"))?;
            }

            #[cfg(target_arch = "wasm32")]
            {
                use base64::Engine;

                let time = Instant::now();
                let width = image.width();
                let height = image.height();

                info!("Encoding {width}x{height} image as {format:?}");

                // encode image in the given format
                let mut buf = std::io::Cursor::new(Vec::new());
                if let Err(_) = image::write_buffer_with_format(
                    &mut buf,
                    &self.last_render,
                    width,
                    height,
                    image::ColorType::Rgba8,
                    format,
                ) {
                    // image format likely doesn't support transparency
                    let buffer: Vec<u8> = self
                        .last_render
                        .chunks_exact(4)
                        .flat_map(|v| &v[0..3])
                        .cloned()
                        .collect();
                    buf = std::io::Cursor::new(Vec::new());
                    image::write_buffer_with_format(
                        &mut buf,
                        &buffer,
                        width,
                        height,
                        image::ColorType::Rgb8,
                        format,
                    )
                    .map_err(|e| format!("failed to encode image: {e}"))?;
                }

                // encode image file as data url and send to frontend
                let data = base64::engine::general_purpose::STANDARD.encode(&buf.into_inner());
                return Ok(Some(BackendEvent::SaveData(
                    time.elapsed(),
                    format!("data:{};base64,{data}", format.to_mime_type()),
                    format,
                )));
            }
        }

        Ok(None)
    }

    fn load_file(
        &mut self,
        path: &Path,
        #[cfg(target_arch = "wasm32")] bytes: Vec<u8>,
    ) -> Result<Option<BackendEvent>, String> {
        let time = Instant::now();

        #[cfg(not(target_arch = "wasm32"))]
        let image = image::open(path);
        #[cfg(target_arch = "wasm32")]
        let image = image::load_from_memory(&bytes);
        let image = image.map_err(|e| e.to_string())?.to_rgba8();

        // hash image
        self.hasher = DefaultHasher::new();
        image.hash(&mut self.hasher);

        let frame = image.to_vec().into();
        let dim = (image.height(), image.width());
        self.current_image = Some(image);

        Ok(Some(BackendEvent::SetImage {
            time: time.elapsed(),
            source: ImageSource::Image(path.to_path_buf()),
            image: frame,
            dim,
        }))
    }

    /// Apply a palette to the currently loaded image
    fn apply_palette(
        &mut self,
        palette: Vec<[u8; 3]>,
        common: Common,
        args: LutAlgorithmArgs,
        abort: Arc<AtomicBool>,
    ) -> Result<Option<BackendEvent>, String> {
        let time = Instant::now();

        let Some(mut image) = self.current_image.clone() else {
            // do nothing if no image is loaded
            return Ok(None);
        };

        // hash arguments with existing image hash
        let mut hasher = self.hasher.clone();
        palette.hash(&mut hasher);
        common.hash(&mut hasher);
        args.hash(&mut hasher);
        let hash = hasher.finish();

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

        // generate lut from arguments
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
            LutAlgorithmArgs::GaussianBlur { args } => {
                lutgen::interpolation::GaussianBlurRemapper::new(
                    &palette,
                    *args.radius,
                    *common.lum_factor,
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
        .ok_or("Cancelled generating hald clut".to_string())?;

        // remap image
        lutgen::identity::correct_image_with_level(&mut image, &lut, common.level);
        self.last_render = image.to_vec().into();

        Ok(Some(BackendEvent::SetImage {
            time: time.elapsed(),
            source: ImageSource::Edited(hash),
            image: self.last_render.clone(),
            dim: (image.height(), image.width()),
        }))
    }
}

#[cfg(target_arch = "wasm32")]
impl gloo_worker::Worker for Worker {
    type Input = FrontendEvent;
    type Output = BackendEvent;
    type Message = ();

    fn create(_scope: &gloo_worker::WorkerScope<Self>) -> Self {
        Worker {
            current_image: None,
            hasher: DefaultHasher::new(),
            last_render: Default::default(),
        }
    }

    fn received(
        &mut self,
        scope: &gloo_worker::WorkerScope<Self>,
        msg: FrontendEvent,
        id: gloo_worker::HandlerId,
    ) {
        if let Some(event) = self.handle_event(msg) {
            scope.respond(id, event);
        }
    }

    fn update(&mut self, _scope: &gloo_worker::WorkerScope<Self>, _msg: Self::Message) {}
}
