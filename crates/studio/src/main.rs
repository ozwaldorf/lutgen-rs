#![warn(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use bpaf::Bpaf;
use egui_file_dialog::FileDialog;
use log::LevelFilter;

use crate::state::{LutAlgorithm, UiState};
use crate::worker::{LutAlgorithmArgs, WorkerHandle};

mod palette;
mod state;
mod ui;
mod utils;
mod worker;

const IMAGE_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpg", "jpeg", "png", "pnm", "qoi",
    "tga", "tiff", "webp",
];

#[derive(Bpaf)]
#[bpaf(options, version)]
pub struct Cli {
    /// Verbosity level, repeat for more (ie, -vvv for maximum logging)
    #[bpaf(
        short, req_flag(()), count,
        map(|l| {
            use LevelFilter::*;
            [Warn, Info, Debug, Trace][l.clamp(0, 3)]
        })
    )]
    verbosity: LevelFilter,
}

pub struct App {
    state: UiState,
    worker: WorkerHandle,
    scene_rect: egui::Rect,

    pub open_picker: FileDialog,
    pub save_picker: FileDialog,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Theming
        cc.egui_ctx.set_visuals(egui::Visuals {
            dark_mode: true,
            // bg
            window_fill: egui::Color32::from_rgb(0x16, 0x16, 0x16),
            code_bg_color: egui::Color32::from_rgb(0x0b, 0x0b, 0x0b),
            faint_bg_color: egui::Color32::from_rgb(0x26, 0x26, 0x26),
            extreme_bg_color: egui::Color32::from_rgb(0x39, 0x39, 0x39),
            // fg
            override_text_color: Some(egui::Color32::from_rgb(0xf4, 0xf4, 0xf4)),
            hyperlink_color: egui::Color32::from_rgb(0x45, 0x89, 0xff),
            warn_fg_color: egui::Color32::from_rgb(0xfd, 0xdc, 0x69),
            error_fg_color: egui::Color32::from_rgb(0xfa, 0x4d, 0x56),
            ..Default::default()
        });

        // Load previous app state (if any).
        let state = cc
            .storage
            .and_then(|storage| eframe::get_value::<UiState>(storage, eframe::APP_KEY))
            .unwrap_or_default();

        // Spawn background worker thread
        let worker = WorkerHandle::new(cc.egui_ctx.clone());
        let mut this = Self {
            state,
            worker,
            scene_rect: egui::Rect::ZERO,
            open_picker: FileDialog::new()
                .add_file_filter_extensions("Images", IMAGE_EXTENSIONS.to_vec())
                .default_file_filter("Images")
                .title("Open Image"),
            save_picker: FileDialog::new().title("Save Image As"),
        };

        // setup save extensions
        for &ext in IMAGE_EXTENSIONS {
            this.save_picker = this.save_picker.add_save_extension(ext, ext);
        }
        this.save_picker = this.save_picker.default_save_extension("png");

        // Load last opened image and apply settings
        if let Some(path) = &this.state.current_image {
            this.worker.load_file(path);
            this.apply();
        }

        this
    }

    /// Collect arguments and send apply request to the worker
    pub fn apply(&mut self) {
        let args = match self.state.current_alg {
            LutAlgorithm::GaussianRbf => LutAlgorithmArgs::GaussianRbf {
                rbf: self.state.common_rbf,
                args: self.state.guassian_rbf,
            },
            LutAlgorithm::ShepardsMethod => LutAlgorithmArgs::ShepardsMethod {
                rbf: self.state.common_rbf,
                args: self.state.shepards_method,
            },
            LutAlgorithm::GaussianSampling => LutAlgorithmArgs::GaussianSampling {
                args: self.state.guassian_sampling,
            },
            LutAlgorithm::NearestNeighbor => LutAlgorithmArgs::NearestNeighbor,
        };
        self.worker
            .apply_palette(self.state.palette.clone(), self.state.common, args);
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state)
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle any incoming events from the backend
        if let Some(event) = self.worker.poll_event() {
            self.state.handle_event(ctx, event);
        }

        self.open_picker.update(ctx);
        if let Some(path) = self.open_picker.take_picked() {
            self.worker.load_file(&path);
            self.state.current_image = Some(path);
        }

        self.save_picker.update(ctx);
        if let Some(path) = self.save_picker.take_picked() {
            self.worker.save_as(path);
        }

        // Show UI
        self.show(ctx);
    }
}

fn main() -> eframe::Result {
    let cli = cli().run();
    env_logger::builder()
        .filter_level(cli.verbosity)
        .parse_env("RUST_LOG")
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/logo.png")[..])
                    .expect("Failed to load icon"),
            ),
        persist_window: true,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "lutgen-studio",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
