use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;

use crate::spotify::{self, SavedCredentials};

#[derive(Debug)]
pub struct SpotifyWidget {
    client: Arc<Mutex<Option<spotify::Client>>>,
    currently_playing: Arc<Mutex<Option<spotify::PlaybackState>>>,
    last_poll_time: Arc<Mutex<SystemTime>>,
}

impl SpotifyWidget {
    pub fn new(credentials: Option<spotify::SavedCredentials>) -> SpotifyWidget {
        let widget = SpotifyWidget {
            client: Arc::new(Mutex::new(credentials.map(spotify::Client::from_saved))),
            currently_playing: Arc::new(Mutex::new(None)),
            last_poll_time: Arc::new(Mutex::new(SystemTime::now())),
        };

        // poller
        let client_for_poller = Arc::clone(&widget.client);
        let currently_playing_for_poller = Arc::clone(&widget.currently_playing);
        let last_poll_time_for_poller = Arc::clone(&widget.last_poll_time);
        thread::spawn(move || {
            loop {
                {
                    let client = client_for_poller.lock().unwrap();
                    if let Some(client) = client.as_ref() {
                        let new_state = client.get_playback_state();
                        let mut currently_playing = currently_playing_for_poller.lock().unwrap();
                        *currently_playing = new_state;
                    }
                }

                {
                    let mut last_poll_time = last_poll_time_for_poller.lock().unwrap();
                    *last_poll_time = SystemTime::now();
                }
                thread::sleep(Duration::from_secs(10));
            }
        });

        widget
    }

    pub fn save(&self) -> Option<SavedCredentials> {
        self.client
            .lock()
            .unwrap()
            .as_ref()
            .map(spotify::Client::save)
    }

    pub fn open_authorizer(&self) {
        let client_ref = Arc::clone(&self.client);
        thread::spawn(move || {
            let new_client = spotify::Client::from_scratch();
            let mut old_client = client_ref.lock().unwrap();
            *old_client = Some(new_client);
        });
    }

    fn render_time_till_next_poll(&self, ui: &mut egui::Ui) {
        let last_poll_time = self.last_poll_time.lock().unwrap();
        let seconds_since = SystemTime::now()
            .duration_since(*last_poll_time)
            .unwrap()
            .as_secs();
        let until = 10 - seconds_since;
        ui.small(format!(
            "Next check in {} second{}",
            until,
            if until == 1 { "" } else { "s" }
        ));

        ui.request_repaint_after_secs(1.0);
    }

    fn render_currently_playing(&self, ui: &mut egui::Ui) {
        let currently_playing = self.currently_playing.lock().unwrap();
        let Some(state) = currently_playing.as_ref() else {
            ui.label("No track currently playing");
            self.render_time_till_next_poll(ui);
            return;
        };

        let track = &state.track;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.small("Now playing:");
                ui.label(egui::RichText::new(&track.name).size(16.0));
                ui.label(&track.artists[0].name);
            });

            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                self.render_time_till_next_poll(ui);

                ui.add_space(4.0);
                if ui.button("Skip").clicked() {
                    let client = Arc::clone(&self.client);
                    thread::spawn(move || {
                        let client = client.lock().unwrap();
                        if let Some(client) = client.as_ref() {
                            client.skip_song();
                        }
                    });
                }
            });
        });
    }
}

impl egui::Widget for &SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            {
                let client = self.client.lock().unwrap();
                if client.is_none() {
                    if ui.button("Link Spotify").clicked() {
                        self.open_authorizer();
                    }
                    return;
                };
            }

            self.render_currently_playing(ui);
        })
        .response
    }
}
