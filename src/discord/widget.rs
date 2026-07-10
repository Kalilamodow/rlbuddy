use eframe::egui;

use super::service::{DiscordCommand, DiscordServiceState, DiscordSettings};

pub struct DiscordWidget {
    local_settings: Option<DiscordSettings>,
    send_command: Option<DiscordCommand>,
}

impl DiscordWidget {
    pub fn new() -> Self {
        DiscordWidget {
            local_settings: None,
            send_command: None,
        }
    }

    pub fn get_command(&mut self) -> Option<DiscordCommand> {
        self.send_command.take()
    }

    pub fn logic(&mut self, service_state: DiscordServiceState) {
        self.local_settings = Some(service_state.settings.as_ref().clone())
    }

    fn send_update_settings(&mut self) {
        let Some(settings) = &self.local_settings else {
            return;
        };

        self.send_command = Some(DiscordCommand::UpdateSettings(settings.clone()));
    }
}

impl egui::Widget for &mut DiscordWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            let mut updated = false;

            let Some(settings) = &mut self.local_settings else {
                ui.spinner();
                return;
            };

            if ui.checkbox(&mut settings.disable, "Disabled").changed() {
                updated = true;
            }

            ui.add_enabled_ui(!settings.disable, |ui| {
                if ui
                    .checkbox(&mut settings.hide_score, "Hide score")
                    .changed()
                {
                    updated = true;
                }
            });

            if updated {
                self.send_update_settings();
            }
        })
        .response
    }
}
