use strum::VariantArray;

use crate::palette::DynamicPalette;
use crate::state::LutAlgorithm;
use crate::App;

impl App {
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
                            .selected_text(format!("{}", self.state.palette_selection))
                            .width(ui.available_width() - ui.spacing().item_spacing.x - label_width)
                            .show_ui(ui, |ui| {
                                let val = ui.selectable_value(
                                    &mut self.state.palette_selection,
                                    DynamicPalette::Custom,
                                    "- custom -",
                                );
                                val.gained_focus()
                                    .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));

                                for p in lutgen_palettes::Palette::VARIANTS {
                                    let val = ui.selectable_value(
                                        &mut self.state.palette_selection,
                                        DynamicPalette::Builtin(*p),
                                        p.to_string(),
                                    );
                                    val.clicked().then(|| {
                                        self.state.palette =
                                            self.state.palette_selection.get().to_vec();
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
                            for color in self.state.palette.iter_mut() {
                                res.push(egui::widgets::color_picker::color_edit_button_srgb(
                                    ui, color,
                                ));
                            }
                            for (i, res) in res.iter().enumerate() {
                                if res.changed() {
                                    self.state.palette_selection = DynamicPalette::Custom;
                                    apply = true;
                                }
                                if res.secondary_clicked() {
                                    self.state.palette.remove(i);
                                    self.state.palette_selection = DynamicPalette::Custom;
                                    apply = true;
                                }
                            }

                            if ui
                                .add(egui::Button::new("+").min_size(egui::vec2(40., 0.)))
                                .clicked()
                            {
                                self.state.palette.push([0u8; 3]);
                            };
                        });
                    });

                    ui.horizontal_wrapped(|ui| {
                        let label_width = ui.label("Algorithm:").rect.width();
                        egui::ComboBox::from_id_salt("algorithm")
                            .selected_text(format!("{:?}", self.state.current_alg))
                            .width(ui.available_width() - ui.spacing().item_spacing.x - label_width)
                            .show_ui(ui, |ui| {
                                for alg in LutAlgorithm::VARIANTS {
                                    let val = ui.selectable_value(
                                        &mut self.state.current_alg,
                                        *alg,
                                        alg.to_string(),
                                    );
                                    apply |= val.clicked();
                                    val.gained_focus()
                                        .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));
                                }
                            });
                    });

                    ui.group(|ui| {
                        ui.heading("Common Arguments");

                        ui.add_space(10.);

                        ui.label("Hald-Clut Level");
                        let res = ui.add(egui::Slider::new(&mut self.state.common.level, 4..=16));
                        apply |= res.drag_stopped() | res.lost_focus();

                        ui.label("Luminosity Factor");
                        let res = ui.add(egui::Slider::new(
                            self.state.common.lum_factor.as_mut(),
                            0.0001..=2.,
                        ));
                        apply |= res.drag_stopped() | res.lost_focus();

                        // shared rbf args
                        match self.state.current_alg {
                            LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                                ui.separator();
                                ui.heading("Rbf Arguments");
                                ui.add_space(10.);

                                ui.label("Nearest Colors");
                                let res = ui.add(
                                    egui::Slider::new(&mut self.state.common_rbf.nearest, 0..=32)
                                        .step_by(1.),
                                );
                                apply |= res.drag_stopped() | res.lost_focus();

                                apply |= ui
                                    .checkbox(
                                        &mut self.state.common_rbf.preserve,
                                        "Preserve Luminosity",
                                    )
                                    .changed();
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
                                    self.state.guassian_rbf.shape.as_mut(),
                                    0.0001..=512.,
                                ));
                                apply |= res.drag_stopped() | res.lost_focus();
                            },
                            LutAlgorithm::ShepardsMethod => {
                                ui.separator();
                                ui.heading("Shepard's Method Arguments");
                                ui.add_space(10.);

                                ui.label("Power");
                                let res = ui.add(egui::Slider::new(
                                    self.state.shepards_method.power.as_mut(),
                                    0.0001..=64.,
                                ));
                                apply |= res.drag_stopped() | res.lost_focus();
                            },
                            LutAlgorithm::GaussianSampling => {
                                ui.separator();
                                ui.heading("Guassian Sampling Arguments");
                                ui.add_space(10.);

                                ui.label("Mean");
                                let res = ui.add(egui::Slider::new(
                                    self.state.guassian_sampling.mean.as_mut(),
                                    -128.0..=128.,
                                ));
                                apply |= res.drag_stopped() | res.lost_focus();

                                ui.label("Standard Deviation");
                                let res = ui.add(egui::Slider::new(
                                    self.state.guassian_sampling.std_dev.as_mut(),
                                    1.0..=128.,
                                ));
                                apply |= res.drag_stopped() | res.lost_focus();

                                ui.label("Iterations");
                                let res = ui.add(egui::Slider::new(
                                    &mut self.state.guassian_sampling.iterations,
                                    1..=1024,
                                ));
                                apply |= res.drag_stopped() | res.lost_focus();

                                ui.label("RNG Seed");
                                let res = ui.add(
                                    egui::DragValue::new(&mut self.state.guassian_sampling.seed)
                                        .speed(2i32.pow(20)),
                                );
                                apply |= res.drag_stopped() | res.lost_focus();
                            },
                            _ => {},
                        }
                    });

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Reset").clicked() {
                                self.state.reset_current_args();
                                self.apply();
                            }
                            if ui.button("Copy CLI Arguments").clicked() {
                                let args = self.state.cli_args();
                                ui.ctx()
                                    .copy_text("lutgen apply ".to_string() + &args.join(" "));
                            }
                        });
                    });

                    ui.style_mut().spacing.slider_width = ui.available_width() - 16.;
                });
            });
        apply
    }
}
