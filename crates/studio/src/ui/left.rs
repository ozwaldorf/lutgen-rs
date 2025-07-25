use std::rc::Rc;

use strum::VariantArray;

use crate::palette::{lutgen_dir, DynamicPalette};
use crate::state::LutAlgorithm;
use crate::App;

pub struct PaletteFilterBox {
    items: Vec<Rc<DynamicPalette>>,
    idx: usize,
    filter: String,
    filtered: Vec<Rc<DynamicPalette>>,
}

impl PaletteFilterBox {
    pub fn new(current: &DynamicPalette) -> Self {
        let items: Vec<_> = DynamicPalette::get_all()
            .unwrap()
            .into_iter()
            .map(Rc::new)
            .collect();

        Self {
            filter: String::new(),
            filtered: items.clone(),
            idx: items
                .iter()
                .position(|v| **v == *current)
                .unwrap_or_default(),
            items,
        }
    }

    pub fn reindex(&mut self, current: &DynamicPalette) {
        self.items = DynamicPalette::get_all()
            .unwrap()
            .into_iter()
            .map(Rc::new)
            .collect();
        self.filter();
        self.idx = self
            .filtered
            .iter()
            .position(|v| **v == *current)
            .unwrap_or_default();
    }

    fn filter(&mut self) {
        if self.filter.is_empty() {
            self.filtered = self.items.clone();
        } else {
            self.filtered = self
                .items
                .iter()
                .filter(|palette| palette.as_str().contains(&self.filter.to_lowercase()))
                .cloned()
                .collect();
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, current: &mut DynamicPalette) -> egui::Response {
        let mut apply = false;
        let mut res = ui
            .group(|ui| {
                egui::Resize::default()
                    .resizable([true, false])
                    .min_width(ui.available_width())
                    .max_width(ui.available_width())
                    .with_stroke(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if egui::TextEdit::singleline(&mut self.filter)
                                .hint_text("Search Palettes ...")
                                .desired_width(ui.available_width() - 54.)
                                .show(ui)
                                .response
                                .changed()
                            {
                                // update filtered items
                                self.filter();
                                if !self.filtered.is_empty() {
                                    self.idx = 0;
                                    *current = (*self.filtered[self.idx]).clone();
                                }
                            }

                            if ui.button("<").clicked() && !self.filtered.is_empty() {
                                if self.idx == 0 {
                                    self.idx = self.filtered.len() - 1;
                                } else {
                                    self.idx -= 1;
                                }
                                *current = (*self.filtered[self.idx]).clone();
                                apply = true;
                            }
                            if ui.button(">").clicked() && !self.filtered.is_empty() {
                                if self.idx >= self.filtered.len() - 1 {
                                    self.idx = 0;
                                } else {
                                    self.idx += 1;
                                }
                                *current = (*self.filtered[self.idx]).clone();
                                apply = true;
                            }
                        });

                        ui.separator();

                        egui::ScrollArea::new([true, true])
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (i, palette) in self.filtered.iter().enumerate() {
                                    let selected = (**palette).clone();
                                    let res = ui.add(
                                        egui::Button::selectable(
                                            *current == selected,
                                            palette.as_str(),
                                        )
                                        .min_size(egui::Vec2::new(ui.available_width(), 16.)),
                                    );
                                    // scroll when item is focused
                                    res.gained_focus()
                                        .then(|| ui.scroll_to_cursor(Some(egui::Align::Center)));
                                    // scroll when we applied above
                                    if apply && *current == **palette {
                                        res.request_focus();
                                        ui.scroll_to_cursor(Some(egui::Align::Center));
                                    }
                                    if res.clicked() {
                                        *current = selected;
                                        self.idx = i;
                                        apply = true;
                                    }
                                }
                            });
                    })
            })
            .response;

        // If we need to apply a new palette, mark the response as changed
        if apply {
            res.mark_changed();
        }

        res
    }
}

pub struct PaletteEditor {
    name: String,
}

impl PaletteEditor {
    pub fn new(current: &DynamicPalette) -> Self {
        Self {
            name: current.to_string(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        palette: &mut Vec<[u8; 3]>,
        current: &mut DynamicPalette,
    ) -> [bool; 2] {
        // color palette
        let mut apply = false;
        let mut saved = false;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let enabled = matches!(current, DynamicPalette::Custom(_));
                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut self.name)
                        .desired_width(ui.available_width() - 49.),
                );

                if ui.add_enabled(enabled, egui::Button::new("save")).clicked() {
                    *current = DynamicPalette::Custom(self.name.clone());
                    current.save(palette).unwrap();
                    saved = true;
                }
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().interact_size.x =
                    calculate_width(ui.available_width() + 7., 40., ui.spacing().item_spacing.x);

                let mut res = Vec::new();
                for color in palette.iter_mut() {
                    res.push(egui::widgets::color_picker::color_edit_button_srgb(
                        ui, color,
                    ));
                }
                for (i, res) in res.iter().enumerate() {
                    if res.changed() {
                        if matches!(current, DynamicPalette::Builtin(_)) {
                            let name = current.to_string() + "-custom";
                            self.name = name.clone();
                            *current = DynamicPalette::Custom(name);
                        }
                        apply = true;
                    }
                    if res.secondary_clicked() {
                        if matches!(current, DynamicPalette::Builtin(_)) {
                            let name = current.to_string() + "-custom";
                            self.name = name.clone();
                            *current = DynamicPalette::Custom(name);
                        }
                        palette.remove(i);
                        apply = true;
                    }
                }

                if ui
                    .add(egui::Button::new("+").min_size(ui.spacing().interact_size))
                    .clicked()
                {
                    if matches!(current, DynamicPalette::Builtin(_)) {
                        let name = current.to_string() + "-custom";
                        self.name = name.clone();
                        *current = DynamicPalette::Custom(name);
                    }
                    palette.push([0u8; 3]);
                    apply = true;
                };
            });
        });
        [apply, saved]
    }
}

/// Calculates optimal button width for a wrapped grid with padding.
pub fn calculate_width(width: f32, target: f32, padding: f32) -> f32 {
    if width <= 0.0 || target <= 0.0 {
        return 0.0;
    }

    let target_with_padding = target + padding;
    if width < target_with_padding {
        return (width - padding).max(0.0);
    }

    let buttons_that_fit = (width / target_with_padding).round();
    if buttons_that_fit <= 0.0 {
        return (width - padding).max(0.0);
    }

    (width / buttons_that_fit) - padding
}

impl App {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        // side panel for lut args
        egui::SidePanel::left("args")
            .resizable(true)
            .min_width(214.)
            .show(ctx, |ui| {
                let mut apply = false;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().spacing.slider_width = ui.available_width() - 62.;
                    ui.add_space(4.);

                    // palette menu
                    if self
                        .palette_box
                        .show(ui, &mut self.state.palette_selection)
                        .changed()
                    {
                        self.state.palette = self.state.palette_selection.get().to_vec();
                        self.palette_edit.name = self.state.palette_selection.to_string();
                        apply = true;
                    }

                    // palette editor
                    let [changed, saved] = self.palette_edit.show(
                        ui,
                        &mut self.state.palette,
                        &mut self.state.palette_selection,
                    );
                    apply |= changed;
                    if saved {
                        self.palette_box.reindex(&self.state.palette_selection);
                        self.state.last_event = format!(
                            "Saved custom palette to {}",
                            lutgen_dir().join(&self.palette_edit.name).display()
                        );
                    }

                    // Algorithm arguments
                    ui.group(|ui| {
                        // Algorithm dropdown
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Reset").clicked() {
                                        self.state.reset_current_args();
                                        self.apply();
                                    }
                                    egui::ComboBox::from_id_salt("algorithm")
                                        .selected_text(format!("{:?}", self.state.current_alg))
                                        .width(ui.available_width())
                                        .show_ui(ui, |ui| {
                                            for alg in LutAlgorithm::VARIANTS {
                                                let val = ui.selectable_value(
                                                    &mut self.state.current_alg,
                                                    *alg,
                                                    alg.to_string(),
                                                );
                                                apply |= val.clicked();
                                                val.gained_focus().then(|| {
                                                    ui.scroll_to_cursor(Some(egui::Align::Center))
                                                });
                                            }
                                        });
                                },
                            );
                        });
                        ui.separator();

                        // common args
                        ui.heading("Common Arguments");
                        ui.add_space(5.);

                        ui.label("Hald-Clut Level");
                        let res = ui.add(egui::Slider::new(&mut self.state.common.level, 4..=16));
                        apply |= res.drag_stopped() | res.lost_focus();

                        ui.label("Luminosity Factor");
                        let res = ui.add(egui::Slider::new(
                            self.state.common.lum_factor.as_mut(),
                            0.001..=2.,
                        ));
                        apply |= res.drag_stopped() | res.lost_focus();

                        apply |= ui
                            .checkbox(&mut self.state.common.preserve, "Preserve Luminosity")
                            .changed();

                        // unique algorithm args
                        match self.state.current_alg {
                            LutAlgorithm::GaussianRbf => {
                                ui.separator();
                                ui.heading("Gaussian Arguments");
                                ui.add_space(5.);

                                ui.label("Shape");
                                let res = ui.add(egui::Slider::new(
                                    self.state.guassian_rbf.shape.as_mut(),
                                    0.001..=512.,
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
                                    0.001..=64.,
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
                                    -127.0..=127.,
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

                        // shared rbf args
                        match self.state.current_alg {
                            LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                                ui.label("Nearest Colors");
                                let res = ui.add(
                                    egui::Slider::new(&mut self.state.common_rbf.nearest, 0..=32)
                                        .step_by(1.),
                                );
                                apply |= res.drag_stopped() | res.lost_focus();
                            },
                            _ => {},
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new("Copy CLI Arguments")
                                    .min_size(egui::Vec2::new(ui.available_width(), 16.)),
                            )
                            .clicked()
                        {
                            let args = self.state.cli_args();
                            ui.ctx()
                                .copy_text("lutgen apply ".to_string() + &args.join(" "));
                        }
                    });

                    if apply {
                        self.apply();
                    }
                });
            });
    }
}
