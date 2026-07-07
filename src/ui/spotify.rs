use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::spotify::{self, SavedCredentials};

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

#[derive(Debug)]
pub struct SpotifyWidget {
    client: Arc<Mutex<Option<spotify::Client>>>,
    currently_playing: Arc<Mutex<Option<spotify::PlaybackState>>>,
    last_poll_time: Arc<Mutex<SystemTime>>,
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

        let client = Arc::new(Mutex::new(
            savedata
                .unwrap_or_default()
                .credentials
                .map(spotify::Client::from_saved),
        ));

        let (cmd_tx, cmd_rx) = mpsc::channel();

        let currently_playing = Arc::new(Mutex::new(None));
        let widget = SpotifyWidget {
            last_poll_time: Arc::new(Mutex::new(SystemTime::now())),
            client,
            currently_playing,
            pause_during_replay,
            cmd_tx,
            cmd_rx,
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

    pub fn save(&self) -> SpotifySavedata {
        let credentials = self
            .client
            .lock()
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

impl egui::Widget for &mut SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        while self.pause_during_replay
            && let Ok(cmd) = self.cmd_rx.try_recv()
        {
            println!("received command: {cmd:?}");
            let client = self.client.lock().unwrap();
            if let Some(client) = client.as_ref() {
                match cmd {
                    SpotifyCommand::Play => client.unpause_playback(),
                    SpotifyCommand::Pause => client.pause_playback(),
                }
            }
        }

        ui.vertical(|ui| {
            {
                let client = self.client.lock().unwrap();
                if client.is_none() {
                    if ui.button("Link Spotify").clicked() {
                        self.open_authorizer();
                    }
                    return;
                }
            }

            self.render_currently_playing(ui);
            ui.checkbox(&mut self.pause_during_replay, "Pause during replay");
        })
        .response
    }
}
