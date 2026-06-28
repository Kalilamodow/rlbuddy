use std::{
    collections::HashSet,
    sync::{Arc, RwLock, mpsc},
};

use eframe::egui;
use rdev::{Event, EventType, Key};

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
) {
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

    if state.keys_pressed.contains(&Key::Alt) {
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

pub fn listen_for_hotkey(trigger: mpsc::Sender<bool>, ctx: egui::Context) {
    let state = Arc::new(RwLock::new(HotkeyState::default()));
    if let Err(error) = rdev::listen(move |e| callback(&state, &e, &trigger, &ctx)) {
        println!("Hotkey hook error: {error:?}");
    }
}
