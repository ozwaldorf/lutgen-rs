use crate::App;

impl App {
    pub fn show_about_dialog(&mut self, ctx: &egui::Context) {
        if self.state.show_about {
            let id = egui::ViewportId(egui::Id::new("about"));
            let vp = egui::ViewportBuilder::default()
                .with_title("About Lutgen Studio")
                .with_active(true)
                .with_resizable(false)
                .with_inner_size((400.0, 420.0));

            ctx.show_viewport_immediate(id, vp, |ctx, _| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.);
                            ui.add(egui::Image::from_texture(&self.icon).max_width(100.));
                            ui.add_space(20.);
                            ui.heading(format!("Lutgen Studio v{}", env!("CARGO_PKG_VERSION")));
                            ui.add_space(5.);
                            ui.label(format!(
                                "License {}, Copyright 2025",
                                env!("CARGO_PKG_LICENSE")
                            ));
                            ui.label(env!("CARGO_PKG_AUTHORS"));
                            if ui.link("Source Code (Github)").clicked() {
                                ui.ctx()
                                    .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY")));
                            }

                            ui.add_space(20.);
                            ui.add(egui::Separator::default().shrink(50.));
                            ui.add_space(20.);

                            ui.heading("Basic Help");
                            ui.add_space(20.);
                            ui.label("Images can be opened and saved in the `File` dialog");
                            ui.add_space(5.);
                            ui.label(
                                "Left-click the preview to toggle between original and edited",
                            );
                            ui.add_space(5.);
                            ui.label(
                                "Left-click the palette colors to edit, right-click to delete",
                            );
                        });
                    });
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
