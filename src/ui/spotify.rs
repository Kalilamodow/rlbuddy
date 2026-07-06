use std::{
    sync::{Arc, RwLock},
    thread,
};

use eframe::egui;

use crate::spotify::{self, SavedCredentials};

#[derive(Debug, Default)]
struct SpotifyWidgetData {
    client: Option<spotify::Client>,
}

pub struct SpotifyWidget {
    data: Arc<RwLock<SpotifyWidgetData>>,
}

impl SpotifyWidget {
    pub fn new(credentials: Option<spotify::SavedCredentials>) -> SpotifyWidget {
        SpotifyWidget {
            data: Arc::new(RwLock::new(SpotifyWidgetData {
                client: credentials.map(spotify::Client::from_saved),
            })),
        }
    }

    pub fn save(&self) -> Option<SavedCredentials> {
        self.data
            .read()
            .unwrap()
            .client
            .as_ref()
            .map(spotify::Client::save)
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

            ui.label(format!("Auth: {auth:#?}"));
            if ui.button("Unlink").clicked() {
                data.client = None;
            }
        })
        .response
    }
}
