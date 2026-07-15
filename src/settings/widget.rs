use eframe::egui;

use crate::hotkey::{HotkeyService, HotkeySettingsWidget};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    HotkeySettings,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::HotkeySettings => "Keybind",
            }
        )
    }
}

const ALL_PANELS: [Panel; 1] = [Panel::HotkeySettings];

pub struct SettingsWidget {
    hotkey: HotkeySettingsWidget,
}

impl SettingsWidget {
    pub fn new(hotkey_service: &HotkeyService) -> Self {
        Self {
            hotkey: HotkeySettingsWidget::new(hotkey_service.settings_handle()),
        }
    }
}

impl egui::Widget for &mut SettingsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            for panel in ALL_PANELS {
                ui.add_space(4.0);
                ui.group(|ui| {
                    ui.strong(panel.to_string());

                    match panel {
                        Panel::HotkeySettings => ui.add(&self.hotkey),
                    };
                });
            }
        })
        .response
    }
}
