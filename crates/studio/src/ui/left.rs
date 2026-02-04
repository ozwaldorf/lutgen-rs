use std::ops::RangeInclusive;
use std::rc::Rc;

use strum::VariantArray;

use crate::palette::{lutgen_dir, DynamicPalette};
use crate::state::LutAlgorithm;
use crate::App;

/// Helper to add a labeled slider with dynamic DragValue sizing.
/// The DragValue is measured first, then the slider fills remaining space.
fn labeled_slider<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Num,
    range: RangeInclusive<Num>,
) -> egui::Response {
    ui.label(label);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let drag = ui.add(egui::DragValue::new(value).range(range.clone()));
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.style_mut().spacing.slider_width = ui.available_width();
                let slider = ui.add(egui::Slider::new(value, range).show_value(false));
                drag | slider
            })
            .inner
        })
        .inner
    })
    .inner
}

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
                                .desired_width(ui.available_width() - 55.)
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
                                        .min_size(egui::Vec2::new(ui.available_width() - 1., 16.)),
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
    fn show_settings(&mut self, ui: &mut egui::Ui) -> bool {
        let mut apply = false;
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

            let res = labeled_slider(ui, "Hald-Clut Level", &mut self.state.common.level, 4..=16);
            apply |= res.drag_stopped() | res.lost_focus();
            res.on_hover_text("\
                Hald clut level to generate. Heavy impact on performance for high levels. \n\
                A level of 16 computes a value for the entire sRGB color space.\n\n\
                Range: 4-16",
            );

            let res = labeled_slider(ui, "Luminosity Factor", self.state.common.lum_factor.as_mut(), 0.001..=2.);
            apply |= res.drag_stopped() | res.lost_focus();
            res.on_hover_text("\
                Factor to multiply luminocity values by. \
                Effectively weights the interpolation to prefer more \
                colorful or more greyscale/unsaturated matches.\n\n\
                Tip: Use values below 1.0 for more colorful results, \
                above 1.0 for less colorful results. \
                Extreme values usually are paired with 'Preserve Luminosity'.\n\n\
                Default: 0.7");

            let res = ui
                .checkbox(&mut self.state.common.preserve, "Preserve Luminosity");
            apply |= res.changed();
            res.on_hover_text("\
                Preserve the original image's luminocity values after interpolation. \
                This effectively retains the image's contrast and generally improves gradients.\n\n\
                Default: true");

            // unique algorithm args
            match self.state.current_alg {
                LutAlgorithm::GaussianRbf => {
                    ui.separator();
                    ui.heading("Gaussian Arguments");
                    ui.add_space(5.);

                    let res = labeled_slider(ui, "Shape", self.state.guassian_rbf.shape.as_mut(), 0.001..=512.);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Shape parameter for the default Gaussian RBF interpolation. \
                        Effectively creates more or less blending between colors in the palette.\n\n\
                        Bigger numbers = less blending (closer to original colors)\n\
                        Smaller numbers = more blending (smoother results)\n\n\
                        Default: 128.0");
                },
                LutAlgorithm::ShepardsMethod => {
                    ui.separator();
                    ui.heading("Shepard's Method Arguments");
                    ui.add_space(10.);

                    let res = labeled_slider(ui, "Power", self.state.shepards_method.power.as_mut(), 0.001..=64.);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Power parameter for Shepard's method (Inverse Distance RBF).\n\
                        Higher values give more weight to closer palette colors.\n\n\
                        Default: 4.0");
                },
                LutAlgorithm::GaussianSampling => {
                    ui.separator();
                    ui.heading("Guassian Sampling Arguments");
                    ui.add_space(10.);

                    let res = labeled_slider(ui, "Mean", self.state.guassian_sampling.mean.as_mut(), -127.0..=127.);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Average amount of noise to apply in each iteration. \
                        Controls the bias of the random sampling process, and can lighten \
                        or darken the image overall.\n\n\
                        Default: 0.0\nRange: -127.0 to 127.0");

                    let res = labeled_slider(ui, "Standard Deviation", self.state.guassian_sampling.std_dev.as_mut(), 1.0..=128.);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Standard deviation parameter for the noise applied in each iteration. \
                        Controls how much variation is applied during sampling.\n\n\
                        Default: 20.0");

                    let res = labeled_slider(ui, "Iterations", &mut self.state.guassian_sampling.iterations, 1..=1024);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Number of iterations of noise to apply to each pixel.\n\
                        More iterations = better blending but slower processing.\n\n\
                        Default: 512");

                    ui.label("RNG Seed");
                    let res = ui.add(
                        egui::DragValue::new(&mut self.state.guassian_sampling.seed)
                            .speed(2i32.pow(20)),
                    );
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Seed for the random number generator used in noise generation.\n\n\
                        Default: 42080085");
                },
                LutAlgorithm::GaussianBlur => {
                    ui.separator();
                    ui.heading("Gaussian Blur Arguments");
                    ui.add_space(10.);

                    let res = labeled_slider(ui, "Radius", self.state.gaussian_blur.radius.as_mut(), 1.0..=64.0);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Gaussian blur radius (sigma) applied in OKLab color space.\n\n\
                        Higher values = larger blur kernel = more color blending\n\
                        Lower values = smaller kernel = sharper boundaries\n\n\
                        Default: 8.0");
                },
                _ => {},
            }

            // shared rbf args
            match self.state.current_alg {
                LutAlgorithm::GaussianRbf | LutAlgorithm::ShepardsMethod => {
                    let res = labeled_slider(ui, "Nearest Colors", &mut self.state.common_rbf.nearest, 0..=32);
                    apply |= res.drag_stopped() | res.lost_focus();
                    res.on_hover_text("\
                        Number of nearest colors to consider when interpolating.\n\n\
                        0 = uses all available colors ( O(n) )\n\
                        Lower values = faster processing\n\
                        Higher values = more blending\n\n\
                        Default: 16");
                },
                _ => {},
            }
        });

        ui.horizontal(|ui| {
            let res = ui.add(
                egui::Button::new("Copy CLI Arguments")
                    .min_size(egui::Vec2::new(ui.available_width(), 16.)),
            );
            if res.clicked() {
                let args = self.state.cli_args();
                ui.ctx()
                    .copy_text("lutgen apply ".to_string() + &args.join(" "));
            }
        });

        apply
    }

    pub fn show_sidebar_inner(&mut self, ui: &mut egui::Ui) {
        let mut apply = false;
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

        // settings panel
        apply |= self.show_settings(ui);

        if apply {
            self.apply();
        }
    }

    /// side panel for lut args
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        if !self.inline_layout {
            egui::SidePanel::left("args")
                .resizable(true)
                .min_width(214.)
                .show(ctx, |ui| {
                    ui.take_available_width();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.show_sidebar_inner(ui);
                    });
                });
        }
    }
}
