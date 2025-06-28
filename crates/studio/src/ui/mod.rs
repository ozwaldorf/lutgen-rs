use egui::Context;

use crate::App;

mod about;
mod central;
mod left;
mod status;
mod top;

impl App {
    /// Render ui state
    pub fn show(&mut self, ctx: &Context) {
        // about window
        self.show_about_dialog(ctx);

        // top line
        self.show_topbar(ctx);

        // left sidebar for options
        if self.show_sidebar(ctx) {
            // apply on changes
            self.apply();
        }

        // main app window
        self.show_central_panel(ctx);

        self.show_statusline(ctx);
    }
}
