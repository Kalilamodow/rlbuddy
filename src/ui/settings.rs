use rdev::Key;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use eframe::egui;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hotkey {
    #[default]
    Alt,
    LShift,
    LCtrl,
    Tab,
    Super,
    Disabled,
}

impl Hotkey {
    pub fn to_rdev(&self) -> Option<Key> {
        Some(match self {
            Hotkey::Disabled => return None,
            Hotkey::Alt => Key::Alt,
            Hotkey::LShift => Key::ShiftLeft,
            Hotkey::LCtrl => Key::ControlLeft,
            Hotkey::Tab => Key::Tab,
            Hotkey::Super => Key::MetaLeft,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Hotkey::Disabled => "Disabled",
            Hotkey::Alt => "Alt",
            Hotkey::LShift => "Left Shift",
            Hotkey::LCtrl => "Left Ctrl",
            Hotkey::Tab => "Tab",
            Hotkey::Super => "Windows",
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SettingsState {
    pub hotkey: Hotkey,
}

#[derive(Debug, Default)]
pub struct SettingsWidget {
    settings: Arc<RwLock<SettingsState>>,
}

impl SettingsWidget {
    pub fn from_existing(state: SettingsState) -> SettingsWidget {
        SettingsWidget {
            settings: Arc::new(RwLock::new(state)),
        }
    }

    pub fn clone_state(&self) -> Arc<RwLock<SettingsState>> {
        Arc::clone(&self.settings)
    }
}

impl egui::Widget for &SettingsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut settings = self.settings.write().unwrap();

        ui.vertical_centered_justified(|ui| {
            egui::ComboBox::from_label("Hotkey")
                .selected_text(settings.hotkey.as_str())
                .show_ui(ui, |ui| {
                    for key in [
                        Hotkey::Alt,
                        Hotkey::LShift,
                        Hotkey::LCtrl,
                        Hotkey::Tab,
                        Hotkey::Super,
                        Hotkey::Disabled,
                    ] {
                        let key_str = key.as_str();
                        ui.selectable_value(&mut settings.hotkey, key, key_str);
                    }
                });
        })
        .response
    }
}
