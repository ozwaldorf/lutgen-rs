use crate::ui::scene::Scene;
use crate::App;

impl App {
    fn show_image(&mut self, mut size: egui::Vec2, ui: &mut egui::Ui) {
        if self.state.edited_texture.is_none() && self.state.image_texture.is_none() {
            // no image loaded
            if self.inline_layout {
                size.y /= 4.;
            }
            let (rect, res) = ui.allocate_exact_size(size, egui::Sense::click());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Load image ...",
                egui::FontId::default(),
                egui::Color32::GRAY,
            );
            if res.clicked() {
                self.open_picker.trigger(None);
            }
        } else {
            let rect = if !self.state.show_original
                && let Some(texture) = &self.state.edited_texture
            {
                // display edited image
                let res = ui.add(
                    egui::Image::from_texture(texture)
                        .texture_options(egui::TextureOptions::NEAREST)
                        .max_size(size)
                        .corner_radius(4.0)
                        .sense(egui::Sense::click()),
                );
                if res.clicked() {
                    self.state.show_original = true;
                }
                res.rect
            } else if let Some(texture) = &self.state.image_texture {
                // display original image
                let res = ui.add(
                    egui::Image::from_texture(texture)
                        .texture_options(egui::TextureOptions::NEAREST)
                        .max_size(size)
                        .corner_radius(4.0)
                        .sense(egui::Sense::click()),
                );
                if res.clicked() {
                    if self.state.edited_texture.is_none() {
                        // apply if there's no edited texture to show
                        self.apply();
                    }
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
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(0x16, 0x16, 0x16, 172),
                );
                painter.galley(pos, galley, egui::Color32::WHITE);

                res.rect
            } else {
                unreachable!();
            };

            // paint border
            ui.painter().rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::GRAY),
                egui::StrokeKind::Middle,
            );

            // show spinner if processing
            if self.state.processing {
                ui.painter()
                    .rect_filled(rect, 4.0, egui::Color32::from_black_alpha(64));
                egui::Spinner::new().paint_at(
                    ui,
                    egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(30.)),
                );
            }
        }
    }

    /// Main app panel
    pub fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let size = ui.available_size();
            if !self.inline_layout {
                let mut rect = self.scene_rect;
                Scene::new()
                    .zoom_range(0.95..=f32::INFINITY)
                    .constrain_panning(true)
                    .show(ui, &mut rect, |ui| {
                        self.show_image(size, ui);
                    });
                self.scene_rect = rect;
            } else {
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    self.show_image(size, ui);
                    self.show_sidebar_inner(ui);
                });
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
