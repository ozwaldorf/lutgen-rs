use egui::Context;

use crate::App;

mod about;
mod central;
pub mod left;
pub mod scene;
mod status;
mod top;

impl App {
    /// Render ui state
    pub fn show(&mut self, ctx: &Context) {
        self.show_about_dialog(ctx);
        self.show_topbar(ctx);
        self.show_statusline(ctx);
        self.show_sidebar(ctx);
        self.show_central_panel(ctx);
    }
}
