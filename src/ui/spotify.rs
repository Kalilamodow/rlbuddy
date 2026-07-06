use std::{
    sync::{Arc, RwLock},
    thread,
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::spotify::{self, Authorization};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifyData {
    authorization: Option<spotify::Authorization>,
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
            data.write().unwrap().authorization = Some(Authorization::from_scratch());
        });
    }
}

impl egui::Widget for &SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut data = self.data.write().unwrap();

        ui.vertical_centered_justified(|ui| {
            if let Some(auth) = &data.authorization {
                ui.label(format!("Credentials: {:#?}", auth));
                if ui.button("Unauthorize").clicked() {
                    data.authorization = None;
                }
            } else {
                if ui.button("Authorize").clicked() {
                    self.open_authorizer();
                }
            }
        })
        .response
    }
}
