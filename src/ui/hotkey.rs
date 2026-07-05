use std::{
    collections::HashSet,
    sync::{Arc, RwLock, mpsc},
    thread,
};

use eframe::egui;
use rdev::{Event, EventType, Key};
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct HotkeyState {
    keys_pressed: HashSet<Key>,
    previous: bool,
}

fn callback(
    state: &Arc<RwLock<HotkeyState>>,
    event: &Event,
    trigger: &mpsc::Sender<bool>,
    ctx: &egui::Context,
    settings: &Arc<RwLock<HotkeySettings>>,
) {
    let Some(hotkey) = settings.read().unwrap().hotkey.to_rdev() else {
        return;
    };

    let mut state = state.write().unwrap();
    match event.event_type {
        EventType::KeyPress(key) => {
            state.keys_pressed.insert(key);
        }
        EventType::KeyRelease(key) => {
            state.keys_pressed.remove(&key);
        }
        _ => {}
    }

    if state.keys_pressed.contains(&hotkey) {
        if !state.previous {
            state.previous = true;
            trigger.send(true).unwrap();
            ctx.request_repaint();
        }
    } else if state.previous {
        state.previous = false;
        trigger.send(false).unwrap();
        ctx.request_repaint();
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Hotkey {
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    hotkey: Hotkey,
}

pub struct HotkeyWidget {
    settings: Arc<RwLock<HotkeySettings>>,
    trigger: mpsc::Sender<bool>,
    ctx: egui::Context,
}

impl HotkeyWidget {
    pub fn new(
        trigger: mpsc::Sender<bool>,
        ctx: egui::Context,
        settings: Option<HotkeySettings>,
    ) -> HotkeyWidget {
        HotkeyWidget {
            settings: Arc::new(RwLock::new(settings.unwrap_or_default())),
            trigger,
            ctx,
        }
    }

    pub fn start_listening(&self) {
        let trigger = self.trigger.clone();
        let ctx = self.ctx.clone();
        let settings = Arc::clone(&self.settings);

        thread::spawn(move || {
            let state = Arc::new(RwLock::new(HotkeyState::default()));
            if let Err(error) =
                rdev::listen(move |e| callback(&state, &e, &trigger, &ctx, &settings))
            {
                println!("Hotkey hook error: {error:?}");
            }
        });
    }

    pub fn get_settings(&self) -> Arc<RwLock<HotkeySettings>> {
        Arc::clone(&self.settings)
    }
}

impl egui::Widget for &HotkeyWidget {
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
