use std::{
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

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
    settings: Arc<RwLock<RichPresenceSettings>>,
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

        DiscordWidget {
            presence,
            settings: Arc::new(RwLock::new(settings)),
        }
    }

    pub fn clone_settings(&self) -> RichPresenceSettings {
        self.settings.read().unwrap().clone()
    }
}

impl egui::Widget for &DiscordWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut settings = self.settings.write().unwrap();

        ui.vertical_centered_justified(|ui| {
            if ui.checkbox(&mut settings.disable, "Disabled").changed() {
                if settings.disable {
                    self.presence.lock().unwrap().disconnect();
                } else {
                    self.presence.lock().unwrap().connect();
                }
            }

            ui.add_enabled_ui(!settings.disable, |ui| {
                if ui
                    .checkbox(&mut settings.hide_score, "Hide score")
                    .changed()
                {
                    if settings.hide_score {
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
