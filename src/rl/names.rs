// Basically for uncensoring names

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
};

use eframe::egui;
use serde::Deserialize;

const API_URL: &str = "https://mmr.kmdw.dev/get-profile";

#[derive(Debug, Deserialize)]
struct GetProfileResponse {
    name: String,
    // id: String,
    // state: String,
}

pub struct NameAPI {
    // platform id -> loaded
    cache: Arc<RwLock<HashMap<String, Option<Arc<String>>>>>,
    context: egui::Context,
}

impl NameAPI {
    pub fn new(context: egui::Context) -> NameAPI {
        NameAPI {
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

            let Ok(mut response) = super::utils::get_with_retries::<3>(&url) else {
                return;
            };

            let response: GetProfileResponse = response.body_mut().read_json().unwrap();
            let mut current = current.write().unwrap();
            current.insert(player_id, Some(Arc::new(response.name)));
            context.request_repaint();
        });

        None
    }
}
