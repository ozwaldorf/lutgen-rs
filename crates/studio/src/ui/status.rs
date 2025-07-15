use egui::Context;

use crate::App;

impl App {
    pub fn show_statusline(&self, ctx: &Context) {
        // statusline with events and other info
        egui::TopBottomPanel::bottom("statusline").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.state.last_event);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(tex) = &self.state.image_texture {
                        let [w, h] = tex.size();
                        ui.label(format!("{w}x{h}"));
                    }
                    ui.separator();
                    if let Some(path) = &self.state.current_image {
                        ui.label(path.display().to_string());
                    }
                    ui.separator();
                });
            });
        });
    }
}
