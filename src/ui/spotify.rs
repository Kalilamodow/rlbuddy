use std::{
    sync::{Arc, Mutex, RwLock, mpsc},
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::spotify::{self, SavedCredentials};

const SPOTIFY_UPDATE_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum SpotifyCommand {
    Play,
    Pause,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifySavedata {
    credentials: Option<SavedCredentials>,
    pause_during_replay: bool,
}

fn request_new_state(
    client: Arc<RwLock<Option<spotify::Client>>>,
    currently_playing: Arc<Mutex<Option<spotify::PlaybackState>>>,
) {
    thread::spawn(move || {
        let client = client.read().unwrap();
        if let Some(client) = client.as_ref() {
            let new_state = client.get_playback_state();
            let mut currently_playing = currently_playing.lock().unwrap();
            *currently_playing = new_state;
        }
    });
}

#[derive(Debug)]
pub struct SpotifyWidget {
    client: Arc<RwLock<Option<spotify::Client>>>,
    currently_playing: Arc<Mutex<Option<spotify::PlaybackState>>>,
    last_updated_at: SystemTime,
    pause_during_replay: bool,

    cmd_tx: mpsc::Sender<SpotifyCommand>,
    cmd_rx: mpsc::Receiver<SpotifyCommand>,
}

impl SpotifyWidget {
    pub fn new(savedata: Option<SpotifySavedata>) -> SpotifyWidget {
        #[allow(clippy::manual_is_variant_and)]
        let pause_during_replay = savedata
            .as_ref()
            .map(|s| s.pause_during_replay)
            .unwrap_or_default();

        let client = Arc::new(RwLock::new(
            savedata
                .unwrap_or_default()
                .credentials
                .map(spotify::Client::from_saved),
        ));

        let (cmd_tx, cmd_rx) = mpsc::channel();

        let currently_playing = Arc::new(Mutex::new(None));
        SpotifyWidget {
            last_updated_at: SystemTime::now(),
            client,
            currently_playing,
            pause_during_replay,
            cmd_tx,
            cmd_rx,
        }
    }

    pub fn save(&self) -> SpotifySavedata {
        let credentials = self
            .client
            .read()
            .unwrap()
            .as_ref()
            .map(spotify::Client::save);

        SpotifySavedata {
            credentials,
            pause_during_replay: self.pause_during_replay,
        }
    }

    pub fn cmd(&self) -> mpsc::Sender<SpotifyCommand> {
        self.cmd_tx.clone()
    }

    pub fn open_authorizer(&self) {
        let client_ref = Arc::clone(&self.client);
        thread::spawn(move || {
            let new_client = spotify::Client::from_scratch();
            let mut old_client = client_ref.write().unwrap();
            *old_client = Some(new_client);
        });
    }

    fn render_time_till_next_poll(&self, ui: &mut egui::Ui) {
        let seconds_since = SystemTime::now()
            .duration_since(self.last_updated_at)
            .unwrap();

        let until_secs = SPOTIFY_UPDATE_INTERVAL
            .checked_sub(seconds_since)
            .unwrap_or_default()
            .as_secs();

        ui.small(format!(
            "Next check in {} second{}",
            until_secs,
            if until_secs == 1 { "" } else { "s" }
        ));

        ui.request_repaint_after_secs(1.0);
    }

    fn render_currently_playing(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut has_song = true;

            ui.vertical(|ui| {
                let currently_playing = self.currently_playing.lock().unwrap();
                let Some(state) = currently_playing.as_ref() else {
                    has_song = false;
                    ui.label("No track currently playing");
                    return;
                };

                let track = &state.track;
                ui.small("Now playing:");
                ui.label(egui::RichText::new(&track.name).size(16.0));
                ui.label(&track.artists[0].name);
            });

            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                self.render_time_till_next_poll(ui);
                if !has_song {
                    return;
                }

                ui.add_space(4.0);
                if ui.button("Skip").clicked() {
                    let client_lock = Arc::clone(&self.client);
                    let currently_playing = Arc::clone(&self.currently_playing);
                    thread::spawn(move || {
                        let client_guard = client_lock.read().unwrap();
                        if let Some(client) = client_guard.as_ref() {
                            client.skip_song();

                            drop(client_guard);
                            request_new_state(client_lock, currently_playing);
                        }
                    });

                    self.last_updated_at = SystemTime::now();
                }
            });
        });
    }
}

impl egui::Widget for &mut SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        while self.pause_during_replay
            && let Ok(cmd) = self.cmd_rx.try_recv()
        {
            let client = self.client.read().unwrap();
            if let Some(client) = client.as_ref() {
                match cmd {
                    SpotifyCommand::Play => client.unpause_playback(),
                    SpotifyCommand::Pause => client.pause_playback(),
                }
            }
        }

        if SystemTime::now()
            .duration_since(self.last_updated_at)
            .unwrap()
            >= SPOTIFY_UPDATE_INTERVAL
        {
            request_new_state(
                Arc::clone(&self.client),
                Arc::clone(&self.currently_playing),
            );
            self.last_updated_at = SystemTime::now();
        }

        ui.vertical(|ui| {
            {
                let client = self.client.read().unwrap();
                if client.is_none() {
                    if ui.button("Link Spotify").clicked() {
                        self.open_authorizer();
                    }
                    return;
                }
            }

            self.render_currently_playing(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.checkbox(&mut self.pause_during_replay, "Pause during replay");
            });
        })
        .response
    }
}
