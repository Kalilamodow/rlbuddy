use std::{
    sync::{Arc, RwLock},
    thread,
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::spotify;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifyData {
    client: Option<spotify::Client>,
}

pub struct SpotifyWidget {
    data: Arc<RwLock<SpotifyData>>,
}

impl SpotifyWidget {
    pub fn new(data: Option<SpotifyData>) -> SpotifyWidget {
        SpotifyWidget {
            data: Arc::new(RwLock::new(data.unwrap_or_default())),
        }
    }

    pub fn clone_data(&self) -> SpotifyData {
        self.data.read().unwrap().clone()
    }

    pub fn open_authorizer(&self) {
        let data = Arc::clone(&self.data);
        thread::spawn(move || {
            data.write().unwrap().client = Some(spotify::Client::from_scratch());
        });
    }
}

impl egui::Widget for &SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut data = self.data.write().unwrap();

        ui.vertical(|ui| {
            let Some(auth) = &data.client else {
                if ui.button("Link Spotify").clicked() {
                    self.open_authorizer();
                }
                return;
            };

            ui.label(format!("Auth: {:#?}", auth));
            if ui.button("Unlink").clicked() {
                data.client = None;
            }
        })
        .response
    }
}
