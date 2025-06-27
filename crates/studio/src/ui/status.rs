use egui::Context;

use crate::state::UiState;

impl UiState {
    pub fn show_statusline(&self, ctx: &Context) {
        // statusline with events and other info
        egui::TopBottomPanel::bottom("statusline").show(ctx, |ui| {
            ui.label(&self.last_event);
        });
    }
}
