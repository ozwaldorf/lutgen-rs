use crate::App;

impl App {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn show_update(&self, ui: &mut egui::Ui) {
        if let Some(update) = &self.state.update {
            let [maj, min, pat] = update.version;
            if ui
                .link(format!("Update v{maj}.{min}.{pat} available!"))
                .clicked()
            {
                ui.ctx()
                    .open_url(egui::OpenUrl::new_tab(update.url.clone()));
            }
        }
    }

    pub fn show_topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.add(egui::Image::from_texture(&self.icon).max_height(16.));

                #[cfg(not(target_arch = "wasm32"))]
                ui.label("Lutgen Studio");
                #[cfg(target_arch = "wasm32")]
                ui.label("Lutgen Studio Web");

                ui.menu_button("File", |ui| {
                    if ui.button("🖻  Open").clicked() {
                        self.open_picker.trigger(self.state.current_image.clone());
                        ui.close();
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("💾  Save As").clicked() {
                        self.save_picker.trigger(self.state.current_image.clone());
                        ui.close();
                    }

                    #[cfg(target_arch = "wasm32")]
                    ui.add_enabled_ui(self.state.current_image.is_some(), |ui| {
                        egui::containers::menu::SubMenuButton::new("💾  Export").ui(ui, |ui| {
                            for format in image::ImageFormat::all() {
                                let ext = *format.extensions_str().first().unwrap();
                                if ui.button(ext).clicked() {
                                    self.worker.save_as(format);
                                    self.state.processing = true;
                                    ui.close();
                                }
                            }
                        });
                    });

                    ui.menu_button("🎨  Theme", |ui| {
                        egui::widgets::global_theme_preference_buttons(ui);
                    });
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("ℹ  About").clicked() {
                        self.state.show_about = !self.state.show_about;
                    }

                    if ui.button("🖹  Docs").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab("https://lut.sh"));
                    }
                });

                #[cfg(not(target_arch = "wasm32"))]
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.show_update(ui);
                });
            });
        });
    }
}
