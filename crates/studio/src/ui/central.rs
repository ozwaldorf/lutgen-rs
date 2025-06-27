use crate::state::UiState;

impl UiState {
    pub fn show_central_panel(&mut self, ctx: &egui::Context) {
        // main app panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // current image
            ui.vertical_centered(|ui| {
                let available_size = ui.available_size();
                let rect = if !self.show_original
                    && let Some(texture) = &self.edited_texture
                {
                    let res = ui.add(
                        egui::Image::from_texture(texture)
                            .max_size(available_size)
                            .fit_to_exact_size(available_size)
                            .corner_radius(10.0)
                            .sense(egui::Sense::click()),
                    );
                    if res.clicked() {
                        self.show_original = true;
                    }
                    res.rect
                } else if let Some(texture) = &self.image_texture {
                    let res = ui.add(
                        egui::Image::from_texture(texture)
                            .max_size(available_size)
                            .fit_to_exact_size(available_size)
                            .corner_radius(10.0)
                            .sense(egui::Sense::click()),
                    );
                    if res.clicked() {
                        self.show_original = false;
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
