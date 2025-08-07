#![warn(clippy::all)]

use std::path::PathBuf;

use crate::file_picker::FileDialog;
use crate::state::{LutAlgorithm, UiState};
use crate::ui::left::{PaletteEditor, PaletteFilterBox};
pub use crate::worker::Worker;
use crate::worker::{LutAlgorithmArgs, WorkerHandle};

mod color;
mod file_picker;
mod palette;
mod state;
mod ui;
mod updates;
mod utils;
mod worker;

pub struct App {
    /// Main app state
    state: UiState,
    /// Handle to background worker
    worker: WorkerHandle,
    /// Rect for the image preview scene
    scene_rect: egui::Rect,
    /// Filter box for selecting palettes
    palette_box: PaletteFilterBox,
    palette_edit: PaletteEditor,
    // File pickers
    pub open_picker: FileDialog,
    #[cfg(not(target_arch = "wasm32"))]
    pub save_picker: FileDialog,
    /// Lutgen icon
    icon: egui::TextureHandle,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, input: Option<PathBuf>) -> Self {
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
        let mut state = cc
            .storage
            .and_then(|storage| eframe::get_value::<UiState>(storage, eframe::APP_KEY))
            .unwrap_or_default();

        // Set current image if provided by cli
        if let Some(path) = input {
            state.current_image = Some(path);
        }

        // Manually load texture for lutgen icon
        let image_bytes = include_bytes!("../assets/lutgen.png");
        let image_buf = image::load_from_memory(image_bytes)
            .expect("failed to load lutgen icon")
            .to_rgba8();
        let dim = image_buf.dimensions();
        let icon = cc.egui_ctx.load_texture(
            "lutgen.png",
            egui::ColorImage::from_rgba_unmultiplied([dim.0 as usize, dim.1 as usize], &image_buf),
            egui::TextureOptions::default(),
        );

        // Spawn background worker thread
        let worker = WorkerHandle::spawn(cc.egui_ctx.clone());

        let mut this = Self {
            palette_box: PaletteFilterBox::new(&state.palette_selection),
            palette_edit: PaletteEditor::new(&state.palette_selection),
            scene_rect: egui::Rect::ZERO,
            open_picker: FileDialog::pick(cc.egui_ctx.clone()),
            #[cfg(not(target_arch = "wasm32"))]
            save_picker: FileDialog::save(cc.egui_ctx.clone()),
            state,
            icon,
            worker,
        };

        // Optionally load current image and apply settings right away
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = this.state.current_image.clone() {
            this.worker.load_file(path);
            this.apply();
        }

        this
    }

    /// Collect arguments and send apply request to the worker
    pub fn apply(&mut self) {
        // Show the spinner until we receive an edited image
        self.state.processing = true;
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

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = self.open_picker.poll() {
            self.worker.load_file(path.clone());
            self.state.current_image = Some(path);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(item) = self.save_picker.poll() {
            self.worker.save_as(item);
        }

        #[cfg(target_arch = "wasm32")]
        if let Some((path, bytes)) = self.open_picker.poll() {
            self.worker.load_file(path.clone(), bytes);
            self.state.current_image = Some(path);
            self.apply();
        }

        // Show UI
        self.show(ctx);
    }
}
