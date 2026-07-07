use std::{rc::Rc, sync::Mutex};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::discord::RichPresence;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RichPresenceSettings {
    disable: bool,
    hide_score: bool,
}

pub struct DiscordWidget {
    presence: Rc<Mutex<RichPresence>>,
    settings: RichPresenceSettings,
}

impl DiscordWidget {
    pub fn new(
        presence: Rc<Mutex<RichPresence>>,
        settings: Option<RichPresenceSettings>,
    ) -> DiscordWidget {
        let settings = settings.unwrap_or_default();

        if !settings.disable {
            let mut presence = presence.lock().unwrap();
            presence.connect();

            if settings.hide_score {
                presence.hide_score();
            } else {
                presence.show_score();
            }
        }

        DiscordWidget { presence, settings }
    }

    pub fn clone_settings(&self) -> RichPresenceSettings {
        self.settings.clone()
    }
}

impl egui::Widget for &mut DiscordWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            if ui
                .checkbox(&mut self.settings.disable, "Disabled")
                .changed()
            {
                if self.settings.disable {
                    self.presence.lock().unwrap().disconnect();
                } else {
                    self.presence.lock().unwrap().connect();
                }
            }

            ui.add_enabled_ui(!self.settings.disable, |ui| {
                if ui
                    .checkbox(&mut self.settings.hide_score, "Hide score")
                    .changed()
                {
                    if self.settings.hide_score {
                        self.presence.lock().unwrap().hide_score();
                    } else {
                        self.presence.lock().unwrap().show_score();
                    }
                }
            })
        })
        .response
    }
}
