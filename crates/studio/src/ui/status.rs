use egui::Context;

use crate::App;

impl App {
    pub fn show_statusline(&self, ctx: &Context) {
        // statusline with events and other info
        egui::TopBottomPanel::bottom("statusline").show(ctx, |ui| {
            if self.inline_layout {
                ui.vertical_centered(|ui| {
                    ui.add_space(5.);
                    ui.label(&self.state.last_event);
                    if let Some(tex) = &self.state.image_texture
                        && let Some(path) = &self.state.current_image
                    {
                        ui.separator();
                        let [w, h] = tex.size();
                        let path = path.display().to_string();
                        ui.label(format!("{path} ({w}x{h})"));
                    }
                    ui.add_space(5.);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(&self.state.last_event);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let scale = self.scene_transform.scaling * 100.;
                        ui.label(format!("{scale:.0} %"));
                        ui.separator();

                        if let Some(tex) = &self.state.image_texture {
                            let [w, h] = tex.size();
                            ui.label(format!("{w}x{h}"));
                            ui.separator();
                        }
                        if let Some(path) = &self.state.current_image {
                            let path = path.display().to_string();
                            ui.label(path);
                            ui.separator();
                        }
                    });
                });
            }
        });
    }
}
