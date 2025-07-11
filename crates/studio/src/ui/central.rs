use crate::App;

impl App {
    /// Main app panel
    pub fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::containers::Scene::new().zoom_range(0.1..=128.0).show(
                ui,
                &mut self.scene_rect,
                |ui| {
                    // current image
                    let available_size = ui.available_size();
                    let rect = if !self.state.show_original
                        && let Some(texture) = &self.state.edited_texture
                    {
                        let res = ui.add(
                            egui::Image::from_texture(texture)
                                .texture_options(egui::TextureOptions::NEAREST)
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
                                .texture_options(egui::TextureOptions::NEAREST)
                                .max_size(available_size)
                                .fit_to_exact_size(available_size)
                                .corner_radius(10.0)
                                .sense(egui::Sense::click()),
                        );
                        if res.clicked() {
                            self.state.show_original = false;
                        }

                        // draw "original" indicator
                        let painter = ui.painter();
                        let pos = res.rect.min + egui::Vec2::splat(8.);
                        let galley = painter.layout_no_wrap(
                            "Original".to_string(),
                            egui::TextStyle::Body.resolve(ui.style()),
                            egui::Color32::WHITE,
                        );
                        let text_rect = egui::Rect::from_min_size(pos, galley.size());
                        let padding = egui::Vec2::splat(6.0);
                        let bg_rect = text_rect.expand2(padding);
                        painter.rect_filled(
                            bg_rect,
                            4.0,
                            egui::Color32::from_rgba_unmultiplied(0x16, 0x16, 0x16, 172),
                        );
                        painter.galley(pos, galley, egui::Color32::WHITE);

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
                },
            );

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
