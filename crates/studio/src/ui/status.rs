use egui::Context;

use crate::App;

impl App {
    pub fn show_statusline(&self, ctx: &Context) {
        // statusline with events and other info
        egui::TopBottomPanel::bottom("statusline").show(ctx, |ui| {
            ui.label(&self.state.last_event);
        });
    }
}
