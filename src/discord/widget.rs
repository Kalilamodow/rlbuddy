use eframe::egui;

use crate::common::ReadWriteStateHandle;

use super::service::DiscordSettings;

pub struct DiscordWidget {
    state: ReadWriteStateHandle<DiscordSettings>,
}

impl DiscordWidget {
    pub fn new(state: ReadWriteStateHandle<DiscordSettings>) -> Self {
        DiscordWidget { state }
    }
}

impl egui::Widget for &mut DiscordWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            let mut settings = self.state.write();
            ui.checkbox(&mut settings.disable, "Disabled");

            ui.add_enabled_ui(!settings.disable, |ui| {
                ui.checkbox(&mut settings.hide_score, "Hide score");
            });
        })
        .response
    }
}
