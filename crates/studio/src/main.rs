#![warn(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui::Widget;
use log::{debug, error};
use strum::VariantArray;
use uuid::Uuid;

use crate::palette::DynamicPalette;
use crate::state::{LutAlgorithm, UiState};
use crate::worker::{BackendEvent, Worker, WorkerHandle};

mod palette;
mod state;
mod worker;

pub struct App {
    state: UiState,
    worker: WorkerHandle,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

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
        let worker = Worker::spawn(cc.egui_ctx.clone());

        let mut this = Self { state, worker };

        // Load last opened image and apply settings
        if let Some(path) = &this.state.current_image {
            this.worker.load_file(path);
            this.apply();
        }

        this
    }

    /// Handle any incoming events from the backend
    fn poll_worker_events(&mut self, ctx: &egui::Context) {
        if let Some(event) = self.worker.poll_event() {
            self.state.last_event = event.to_string();
            debug!("{}", self.state.last_event);

            match event {
                BackendEvent::Error(e) => {
                    error!("{e}");
                },
                BackendEvent::PickFile(path_buf, _) => {
                    self.state.current_image = Some(path_buf);
                },
                BackendEvent::SetImage {
                    path,
                    image,
                    dim: (width, height),
                } => {
                    // load image into a new egui texture
                    let texture = ctx.load_texture(
                        format!(
                            "bytes://{}",
                            path.as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or(Uuid::new_v4().to_string())
                        ),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [height as usize, width as usize],
                            &image,
                        ),
                        egui::TextureOptions::default(),
                    );

                    if let Some(path) = path {
                        // for newly opened images from file picker
                        self.state.current_image = Some(path);
                        self.state.image_texture = Some(texture);
                        self.state.show_original = true;
                    } else {
                        // for edited output
                        self.state.edited_texture = Some(texture);
                        self.state.show_original = false;
                    }
                },
                _ => {},
            }
        }
    }

    /// Get the currently set lut arguments for worker requests
    fn apply(&mut self) {
        let args = self.state.lut_args();
        self.worker
            .apply_palette(self.state.palette.clone(), self.state.common, args);
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state)
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker_events(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label("Lutgen Studio");
                ui.add_space(5.);
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.worker.pick_file();
                    }
                    if ui.button("Save As").clicked() {
                        self.worker.save_as();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        // side panel for lut args
        egui::SidePanel::left("args")
            .resizable(true)
            .min_width(150.)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Palette:");
                        egui::ComboBox::from_id_salt("palette")
                            .selected_text(format!("{}", self.state.palette_selection))
                            .show_ui(ui, |ui| {
                                let val = ui.selectable_value(
                                    &mut self.state.palette_selection,
                                    palette::DynamicPalette::Custom,
                                    "- custom -",
                                );
                                val.gained_focus()
                                    .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));

                                for p in lutgen_palettes::Palette::VARIANTS {
                                    let val = ui.selectable_value(
                                        &mut self.state.palette_selection,
                                        palette::DynamicPalette::Builtin(*p),
                                        p.to_string(),
                                    );
                                    val.clicked().then(|| {
                                        self.state.palette =
                                            self.state.palette_selection.get().to_vec();
                                        self.apply()
                                    });
                                    val.gained_focus()
                                        .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));
                                }
                            });
                    });

                    // color palette
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            let mut res = Vec::new();
                            for color in self.state.palette.iter_mut() {
                                res.push(egui::widgets::color_picker::color_edit_button_srgb(
                                    ui, color,
                                ));
                            }
                            for (i, res) in res.iter().enumerate() {
                                if res.changed() {
                                    self.state.palette_selection = DynamicPalette::Custom;
                                    self.apply();
                                }
                                if res.secondary_clicked() {
                                    self.state.palette.remove(i);
                                    self.state.palette_selection = DynamicPalette::Custom;
                                    self.apply();
                                }
                            }

                            if egui::Button::new("+")
                                .min_size(egui::vec2(40., 0.))
                                .ui(ui)
                                .clicked()
                            {
                                self.state.palette.push([0u8; 3]);
                            };
                        });
                    });

                    ui.separator();

                    egui::ComboBox::from_id_salt("algorithm")
                        .width(200.)
                        .selected_text(format!("{:?}", self.state.current_alg))
                        .show_ui(ui, |ui| {
                            for alg in LutAlgorithm::VARIANTS {
                                let val = ui.selectable_value(
                                    &mut self.state.current_alg,
                                    *alg,
                                    alg.to_string(),
                                );
                                val.clicked().then(|| self.apply());
                                val.gained_focus()
                                    .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));
                            }
                        });

                    ui.group(|ui| {
                        ui.heading("Common Arguments");
                        ui.add_space(10.);

                        ui.label("Hald-Clut Level");
                        let res = ui.add(egui::Slider::new(&mut self.state.common.level, 4..=16));
                        if res.drag_stopped() || res.lost_focus() {
                            self.apply()
                        }

                        ui.label("Luminosity Factor");
                        let res = ui.add(egui::Slider::new(
                            &mut self.state.common.lum_factor.0,
                            0.0001..=2.,
                        ));
                        if res.drag_stopped() || res.lost_focus() {
                            self.apply()
                        }

                        // shared rbf args
                        match self.state.current_alg {
                            LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                                ui.separator();
                                ui.heading("Rbf Arguments");
                                ui.add_space(10.);

                                ui.toggle_value(
                                    &mut self.state.common_rbf.preserve,
                                    "Preserve Luminosity",
                                )
                                .changed()
                                .then(|| self.apply());

                                ui.label("Nearest Colors");
                                let res = ui.add(
                                    egui::Slider::new(&mut self.state.common_rbf.nearest, 0..=32)
                                        .step_by(1.),
                                );
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }
                            },
                            _ => {},
                        }

                        // unique algorithm args
                        match self.state.current_alg {
                            LutAlgorithm::GaussianRbf => {
                                ui.separator();
                                ui.heading("Gaussian Arguments");
                                ui.add_space(10.);

                                ui.label("Shape");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.state.guassian_rbf.shape.0,
                                    0.0001..=512.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }
                            },
                            LutAlgorithm::ShepardsMethod => {
                                ui.separator();
                                ui.heading("Shepard's Method Arguments");
                                ui.add_space(10.);

                                ui.label("Power");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.state.shepards_method.power.0,
                                    0.0001..=512.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }
                            },
                            LutAlgorithm::GaussianSampling => {
                                ui.separator();
                                ui.heading("Guassian Sampling Arguments");
                                ui.add_space(10.);

                                ui.label("Mean");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.state.guassian_sampling.mean.0,
                                    -128.0..=128.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }

                                ui.label("Standard Deviation");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.state.guassian_sampling.std_dev.0,
                                    1.0..=128.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }
                                ui.label("Iterations");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.state.guassian_sampling.iterations,
                                    1..=1024,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }

                                ui.label("RNG Seed");
                                let res = ui.add(
                                    egui::DragValue::new(&mut self.state.guassian_sampling.seed)
                                        .speed(2i32.pow(20)),
                                );
                                if res.drag_stopped() || res.lost_focus() {
                                    self.apply()
                                }
                            },
                            _ => {},
                        }
                    });
                });
            });

        // statusline with events and other info
        egui::TopBottomPanel::bottom("statusline").show(ctx, |ui| {
            ui.label(&self.state.last_event);
        });

        // main app panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // current image
            ui.vertical_centered(|ui| {
                let available_size = ui.available_size();

                let rect = if !self.state.show_original
                    && let Some(texture) = &self.state.edited_texture
                {
                    let res = ui.add(
                        egui::Image::from_texture(texture)
                            .max_size(available_size)
                            .fit_to_exact_size(available_size)
                            .corner_radius(10.0)
                            .sense(egui::Sense::click()),
                    );
                    if res.clicked() {
                        self.state.show_original = true;
                    }
                    res.rect
                } else if let Some(texture) = &self.state.image_texture {
                    let res = ui.add(
                        egui::Image::from_texture(texture)
                            .max_size(available_size)
                            .fit_to_exact_size(available_size)
                            .corner_radius(10.0)
                            .sense(egui::Sense::click()),
                    );
                    if res.clicked() {
                        self.state.show_original = false;
                    }
                    res.rect
                } else {
                    let (rect, _response) =
                        ui.allocate_exact_size(available_size, egui::Sense::hover());
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No image loaded",
                        egui::FontId::default(),
                        egui::Color32::GRAY,
                    );
                    rect
                };
                // paint border
                ui.painter().rect_stroke(
                    rect,
                    10.0,
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
fn main() -> eframe::Result {
    env_logger::init();
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
