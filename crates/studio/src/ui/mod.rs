use egui::Context;

use crate::state::UiState;
use crate::worker::WorkerHandle;

mod about;
mod central;
mod left;
mod status;
mod top;

impl UiState {
    /// Render ui state
    pub fn show(&mut self, ctx: &Context, worker: &mut WorkerHandle) {
        // about window
        self.show_about_dialog(ctx);

        // top line
        self.show_topbar(ctx, worker);

        // left sidebar for options
        if self.show_sidebar(ctx) {
            // apply on changes
            self.apply(worker);
        }

        // main app window
        self.show_central_panel(ctx);

        self.show_statusline(ctx);
    }
}
