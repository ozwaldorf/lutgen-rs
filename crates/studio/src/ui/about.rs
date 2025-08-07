use crate::App;

impl App {
    fn show_about(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.);
                ui.add(egui::Image::from_texture(&self.icon).max_width(100.));
                ui.add_space(20.);

                ui.heading(format!("Lutgen Studio v{}", env!("CARGO_PKG_VERSION")));
                ui.add_space(10.);
                #[cfg(not(target_arch = "wasm32"))]
                self.show_update(ui);
                ui.label(format!(
                    "License {}, Copyright 2025",
                    env!("CARGO_PKG_LICENSE")
                ));
                ui.label(env!("CARGO_PKG_AUTHORS"));
                if ui.link("Source Code (Github)").clicked() {
                    ui.ctx()
                        .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY")));
                }

                ui.add_space(15.);
                ui.add(egui::Separator::default().shrink(50.));
                ui.add_space(15.);

                #[cfg(target_arch = "wasm32")]
                {
                    ui.heading("Web Version Note");
                    ui.add_space(10.);
                    ui.label("It is recommended to use the native applications.");
                    ui.label(
                        "The web version is much slower than the cli or native gui, and much \
                        less responsive when tweaking settings or using high level luts",
                    );
                    ui.add_space(15.);
                    ui.add(egui::Separator::default().shrink(50.));
                    ui.add_space(15.);
                }

                ui.heading("Basic Help");
                ui.add_space(10.);
                ui.label("Images can be opened and saved in the top bar");
                ui.label("Left-click the preview to toggle between original and edited");
                ui.label("Left-click the palette colors to edit, right-click to delete");
                ui.add_space(20.);
            });
        });
    }

    pub fn show_about_dialog(&mut self, ctx: &egui::Context) {
        let title = "About Lutgen Studio";
        if ctx.embed_viewports() {
            // Embedded UI
            let mut show = self.state.show_about;
            egui::Window::new(title)
                .open(&mut show)
                .collapsible(false)
                .resizable([false, false])
                .show(ctx, |ui| self.show_about(ui));
            self.state.show_about = show;
        } else if self.state.show_about {
            // Native UI
            let id = egui::ViewportId(egui::Id::new("about"));
            let vp = egui::ViewportBuilder::default()
                .with_title(title)
                .with_active(true)
                .with_resizable(false)
                .with_inner_size((400.0, 420.0));

            ctx.show_viewport_immediate(id, vp, |ctx, _| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.show_about(ui);
                });
                ctx.input(|state| {
                    if state.viewport().close_requested() {
                        self.state.show_about = false;
                    }
                });
            });
        }
    }
}
