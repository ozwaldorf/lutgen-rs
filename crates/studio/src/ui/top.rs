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
                if let Some(path) = self.state.current_image.clone() {
                    if ui.button("Download").clicked() {
                        use base64::Engine;
                        use web_sys::wasm_bindgen::JsCast;

                        // encode image as base64 png data
                        let (image, width, height) = self.state.edited_buffer.clone();
                        if width + height > 2 {
                            let mut buf = std::io::Cursor::new(Vec::new());
                            image::write_buffer_with_format(
                                &mut buf,
                                &image,
                                height,
                                width,
                                image::ColorType::Rgba8,
                                image::ImageFormat::Png,
                            )
                            .unwrap();

                            let data =
                                base64::engine::general_purpose::STANDARD.encode(&buf.into_inner());

                            log::info!("encoded image");
                            // create a download link, click it to trigger, and remove afterwards
                            let win = web_sys::window().expect("failed to get window");
                            let doc = win.document().expect("failed to get document");
                            let body = doc.body().unwrap();
                            let link = doc.create_element("a").expect("failed to create link");

                            link.set_attribute("href", &format!("data:image/png;base64,{data}"))
                                .expect("failed to set data");
                            link.set_attribute("download", &path.display().to_string())
                                .expect("failed to set download name");
                            let link: web_sys::HtmlAnchorElement =
                                web_sys::HtmlAnchorElement::unchecked_from_js(link.into());
                            link.click();
                            link.remove();
                        }
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
