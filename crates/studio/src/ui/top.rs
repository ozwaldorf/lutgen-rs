use egui::include_image;

use crate::state::UiState;
use crate::worker::WorkerHandle;

impl UiState {
    pub fn show_topbar(&mut self, ctx: &egui::Context, worker: &WorkerHandle) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.add(egui::Image::new(include_image!("../../assets/logo.png")).max_height(16.));
                ui.label("Lutgen Studio");
                ui.add_space(5.);
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        worker.pick_file(self.current_image.clone());
                    }
                    if ui.button("Save As").clicked() {
                        worker.save_as(self.current_image.clone());
                    }
                    if ui.button("About").clicked() {
                        self.show_about = true;
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
    }
}
