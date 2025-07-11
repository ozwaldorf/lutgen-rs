use egui::include_image;

use crate::App;

impl App {
    pub fn show_topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.add(egui::Image::new(include_image!("../../assets/logo.png")).max_height(16.));
                ui.label("Lutgen Studio");
                ui.add_space(5.);
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(path) = self.state.current_image.clone() {
                            let config = self.open_picker.config_mut();
                            if let Some(parent) = path.parent() {
                                config.initial_directory = parent.to_path_buf();
                            }
                        }
                        self.open_picker.pick_file();
                        ui.close_menu();
                    }
                    if ui.button("Save As").clicked() {
                        if let Some(path) = self.state.current_image.clone() {
                            let config = self.save_picker.config_mut();
                            if let Some(parent) = path.parent() {
                                config.initial_directory = parent.to_path_buf();
                            }
                            if let Some(file) = path.file_name() {
                                config.default_file_name = file.display().to_string();
                            }
                        }
                        self.save_picker.save_file();
                        ui.close_menu();
                    }
                    if ui.button("About").clicked() {
                        self.state.show_about = true;
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });
    }
}
