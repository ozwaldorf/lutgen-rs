use crate::App;

impl App {
    pub fn show_about_dialog(&mut self, ctx: &egui::Context) {
        if self.state.show_about {
            let id = egui::ViewportId(egui::Id::new("about"));
            let vp = egui::ViewportBuilder::default()
                .with_title("About Lutgen Studio")
                .with_active(true)
                .with_resizable(false)
                .with_inner_size((200.0, 200.0));

            ctx.show_viewport_immediate(id, vp, |ctx, _| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(30.);
                            ui.add(
                                egui::Image::new(egui::include_image!("../../assets/lutgen.png"))
                                    .max_width(100.),
                            );
                            ui.add_space(20.);
                            ui.heading("Lutgen Studio");
                            ui.add_space(5.);
                            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));

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
