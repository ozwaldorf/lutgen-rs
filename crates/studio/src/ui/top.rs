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
                ui.label("Lutgen Studio");
                ui.add_space(5.);

                if ui.button("Open").clicked() {
                    self.open_picker.trigger(self.state.current_image.clone());
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Save As").clicked() {
                    self.save_picker.trigger(self.state.current_image.clone());
                }

                #[cfg(target_arch = "wasm32")]
                if self.state.edited_texture.is_some() {
                    if ui.button("Download").clicked() {
                        self.worker.save_as();
                        self.state.processing = true;
                    }
                }

                if ui.button("About").clicked() {
                    self.state.show_about = !self.state.show_about;
                }

                if ui.button("Docs").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://lut.sh"));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                    #[cfg(not(target_arch = "wasm32"))]
                    self.show_update(ui);
                });
            });
        });
    }
}
