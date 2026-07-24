// platform id -> epic name

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
};

use eframe::egui;
use serde::Deserialize;

const API_URL: &str = "https://mmr.kmdw.dev/player-id-to-epic-name";

#[derive(Debug, Deserialize)]
struct EpicIdResponse {
    name: Option<String>,
}

pub struct EpicIdAPI {
    // platform id -> loaded
    cache: Arc<RwLock<HashMap<String, Option<Arc<String>>>>>,
    context: egui::Context,
}

impl EpicIdAPI {
    pub fn new(context: egui::Context) -> Self {
        Self {
            cache: Arc::<RwLock<HashMap<String, Option<Arc<String>>>>>::default(),
            context,
        }
    }

    pub fn get(&self, player_id: &String) -> Option<Arc<String>> {
        let current = Arc::clone(&self.cache);
        if let Some(existing) = current.read().unwrap().get(player_id) {
            return existing.clone();
        }

        let player_id = player_id.clone();
        let context = self.context.clone();

        let url = format!("{}?playerId={}", API_URL, urlencoding::encode(&player_id));

        thread::spawn(move || {
            {
                let mut current = current.write().unwrap();
                current.insert(player_id.clone(), None);
            }

            let Ok(mut response) = ureq::get(&url).call() else {
                let mut current = current.write().unwrap();
                current.remove(&player_id);
                return;
            };

            let response: EpicIdResponse = response.body_mut().read_json().unwrap();
            let mut current = current.write().unwrap();
            current.insert(player_id, response.name.map(Arc::new));
            context.request_repaint();
        });

        None
    }
}
