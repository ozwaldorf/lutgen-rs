use strum::VariantArray;

use crate::palette::DynamicPalette;
use crate::state::{LutAlgorithm, UiState};

impl UiState {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) -> bool {
        let mut apply = false;
        // side panel for lut args
        egui::SidePanel::left("args")
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().spacing.slider_width = ui.available_width() - 62.;

                    ui.horizontal_wrapped(|ui| {
                        let label_width = ui.label("Palette:").rect.width();
                        egui::ComboBox::from_id_salt("palette")
                            .selected_text(format!("{}", self.palette_selection))
                            .width(ui.available_width() - ui.spacing().item_spacing.x - label_width)
                            .show_ui(ui, |ui| {
                                let val = ui.selectable_value(
                                    &mut self.palette_selection,
                                    DynamicPalette::Custom,
                                    "- custom -",
                                );
                                val.gained_focus()
                                    .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));

                                for p in lutgen_palettes::Palette::VARIANTS {
                                    let val = ui.selectable_value(
                                        &mut self.palette_selection,
                                        DynamicPalette::Builtin(*p),
                                        p.to_string(),
                                    );
                                    val.clicked().then(|| {
                                        self.palette = self.palette_selection.get().to_vec();
                                        apply = true
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
                            for color in self.palette.iter_mut() {
                                res.push(egui::widgets::color_picker::color_edit_button_srgb(
                                    ui, color,
                                ));
                            }
                            for (i, res) in res.iter().enumerate() {
                                if res.changed() {
                                    self.palette_selection = DynamicPalette::Custom;
                                    apply = true;
                                }
                                if res.secondary_clicked() {
                                    self.palette.remove(i);
                                    self.palette_selection = DynamicPalette::Custom;
                                    apply = true;
                                }
                            }

                            if ui
                                .add(egui::Button::new("+").min_size(egui::vec2(40., 0.)))
                                .clicked()
                            {
                                self.palette.push([0u8; 3]);
                            };
                        });
                    });

                    ui.horizontal_wrapped(|ui| {
                        let label_width = ui.label("Algorithm:").rect.width();
                        egui::ComboBox::from_id_salt("algorithm")
                            .selected_text(format!("{:?}", self.current_alg))
                            .width(ui.available_width() - ui.spacing().item_spacing.x - label_width)
                            .show_ui(ui, |ui| {
                                for alg in LutAlgorithm::VARIANTS {
                                    let val = ui.selectable_value(
                                        &mut self.current_alg,
                                        *alg,
                                        alg.to_string(),
                                    );
                                    val.clicked().then(|| apply = true);
                                    val.gained_focus()
                                        .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));
                                }
                            });
                    });

                    ui.group(|ui| {
                        ui.heading("Common Arguments");

                        ui.add_space(10.);

                        ui.label("Hald-Clut Level");
                        let res = ui.add(egui::Slider::new(&mut self.common.level, 4..=16));
                        if res.drag_stopped() || res.lost_focus() {
                            apply = true
                        }

                        ui.label("Luminosity Factor");
                        let res = ui.add(egui::Slider::new(
                            &mut self.common.lum_factor.0,
                            0.0001..=2.,
                        ));
                        if res.drag_stopped() || res.lost_focus() {
                            apply = true
                        }

                        // shared rbf args
                        match self.current_alg {
                            LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                                ui.separator();
                                ui.heading("Rbf Arguments");
                                ui.add_space(10.);

                                ui.label("Nearest Colors");
                                let res = ui.add(
                                    egui::Slider::new(&mut self.common_rbf.nearest, 0..=32)
                                        .step_by(1.),
                                );
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }

                                ui.checkbox(&mut self.common_rbf.preserve, "Preserve Luminosity")
                                    .changed()
                                    .then(|| apply = true);
                            },
                            _ => {},
                        }

                        // unique algorithm args
                        match self.current_alg {
                            LutAlgorithm::GaussianRbf => {
                                ui.separator();
                                ui.heading("Gaussian Arguments");
                                ui.add_space(10.);

                                ui.label("Shape");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.guassian_rbf.shape.0,
                                    0.0001..=512.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }
                            },
                            LutAlgorithm::ShepardsMethod => {
                                ui.separator();
                                ui.heading("Shepard's Method Arguments");
                                ui.add_space(10.);

                                ui.label("Power");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.shepards_method.power.0,
                                    0.0001..=512.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }
                            },
                            LutAlgorithm::GaussianSampling => {
                                ui.separator();
                                ui.heading("Guassian Sampling Arguments");
                                ui.add_space(10.);

                                ui.label("Mean");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.guassian_sampling.mean.0,
                                    -128.0..=128.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }

                                ui.label("Standard Deviation");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.guassian_sampling.std_dev.0,
                                    1.0..=128.,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }
                                ui.label("Iterations");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.guassian_sampling.iterations,
                                    1..=1024,
                                ));
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }

                                ui.label("RNG Seed");
                                let res = ui.add(
                                    egui::DragValue::new(&mut self.guassian_sampling.seed)
                                        .speed(2i32.pow(20)),
                                );
                                if res.drag_stopped() || res.lost_focus() {
                                    apply = true
                                }
                            },
                            _ => {},
                        }
                    });

                    ui.style_mut().spacing.slider_width = ui.available_width() - 16.;
                });
            });
        apply
    }
}
