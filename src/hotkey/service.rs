use std::{collections::HashSet, sync::mpsc, thread};

use rdev::{EventType, Key};
use serde::{Deserialize, Serialize};

use crate::common::{ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectableHotkey {
    #[default]
    Alt,
    LShift,
    LCtrl,
    Tab,
    Super,
    Disabled,
}

impl SelectableHotkey {
    pub fn to_rdev(&self) -> Option<Key> {
        Some(match self {
            SelectableHotkey::Disabled => return None,
            SelectableHotkey::Alt => Key::Alt,
            SelectableHotkey::LShift => Key::ShiftLeft,
            SelectableHotkey::LCtrl => Key::ControlLeft,
            SelectableHotkey::Tab => Key::Tab,
            SelectableHotkey::Super => Key::MetaLeft,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SelectableHotkey::Disabled => "Disabled",
            SelectableHotkey::Alt => "Alt",
            SelectableHotkey::LShift => "Left Shift",
            SelectableHotkey::LCtrl => "Left Ctrl",
            SelectableHotkey::Tab => "Tab",
            SelectableHotkey::Super => "Windows",
        }
    }
}

struct InputManager {
    keys_pressed: HashSet<Key>,
    was_open_before: bool,
    tx: mpsc::Sender<bool>,
    settings: ThreadedReadonlyStateHandle<HotkeySettings>,
}

impl InputManager {
    pub fn new(
        tx: mpsc::Sender<bool>,
        settings: ThreadedReadonlyStateHandle<HotkeySettings>,
    ) -> Self {
        InputManager {
            keys_pressed: HashSet::new(),
            was_open_before: false,
            tx,
            settings,
        }
    }

    pub fn listen(mut self) {
        if let Err(error) = rdev::listen(move |e| self.callback(&e)) {
            println!("Hotkey hook error: {error:?}");
        }
    }

    fn callback(&mut self, event: &rdev::Event) {
        let Some(hotkey) = self.settings.read().key.to_rdev() else {
            return;
        };

        match event.event_type {
            EventType::KeyPress(key) => {
                self.keys_pressed.insert(key);
            }
            EventType::KeyRelease(key) => {
                self.keys_pressed.remove(&key);
            }
            _ => {}
        }

        if self.keys_pressed.contains(&hotkey) {
            if !self.was_open_before {
                self.was_open_before = true;
                self.tx.send(true).unwrap();
            }
        } else if self.was_open_before {
            self.was_open_before = false;
            self.tx.send(false).unwrap();
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub key: SelectableHotkey,
}

pub struct HotkeyService {
    settings: ThreadedReadWriteStateHandle<HotkeySettings>,
    rx: mpsc::Receiver<bool>,
}

impl HotkeyService {
    pub fn new(settings: Option<HotkeySettings>) -> Self {
        let settings = ThreadedReadWriteStateHandle::new(settings.unwrap_or_default());
        let (tx, rx) = mpsc::channel();

        let settings_for_manager = settings.clone();
        thread::spawn(move || {
            let manager =
                InputManager::new(tx, ThreadedReadonlyStateHandle::over(&settings_for_manager));
            manager.listen();
        });

        HotkeyService { settings, rx }
    }

    pub fn update(&self) -> Option<bool> {
        self.rx.try_recv().ok()
    }

    pub fn settings_handle(&self) -> ThreadedReadWriteStateHandle<HotkeySettings> {
        ThreadedReadWriteStateHandle::clone(&self.settings)
    }
}
