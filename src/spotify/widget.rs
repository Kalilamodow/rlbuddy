use std::{
    sync::{Arc, Mutex},
    time::SystemTime,
};

use eframe::egui;

use super::{
    client::PlaybackState,
    service::{SPOTIFY_REFRESH_INTERVAL, SpotifyCommand, SpotifyServiceState, SpotifySettings},
};

#[derive(Debug)]
struct WidgetCache {
    currently_playing: Arc<Mutex<Option<PlaybackState>>>,
    last_updated_at: SystemTime,
    logged_in: bool,
}

#[derive(Debug)]
pub struct SpotifyWidget {
    cache: Option<WidgetCache>,
    local_settings: SpotifySettings, // gets this from service state
    send_command: Option<SpotifyCommand>,
}

impl SpotifyWidget {
    pub fn new() -> SpotifyWidget {
        SpotifyWidget {
            cache: None,
            local_settings: SpotifySettings::default(),
            send_command: None,
        }
    }

    pub fn get_command(&mut self) -> Option<SpotifyCommand> {
        self.send_command.take()
    }

    pub fn logic(&mut self, state: SpotifyServiceState) {
        self.cache = Some(WidgetCache {
            currently_playing: state.playback_state,
            last_updated_at: state.last_updated_at,
            logged_in: state.logged_in,
        });
        self.local_settings = state.settings.as_ref().clone();
    }

    fn render_time_till_next_poll(&self, ui: &mut egui::Ui) {
        let Some(cache) = &self.cache else {
            ui.spinner();
            return;
        };

        let seconds_since = SystemTime::now()
            .duration_since(cache.last_updated_at)
            .unwrap();

        let until_secs = SPOTIFY_REFRESH_INTERVAL
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
                let Some(cache) = &self.cache else {
                    return;
                };

                let currently_playing = cache.currently_playing.lock().unwrap();
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
                    self.send(SpotifyCommand::Skip);
                }
            });
        });
    }

    fn send_settings_update(&mut self) {
        self.send(SpotifyCommand::UpdateSettings(self.local_settings.clone()));
    }

    fn send(&mut self, cmd: SpotifyCommand) {
        self.send_command = Some(cmd);
    }
}

impl egui::Widget for &mut SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            let Some(cache) = &self.cache else {
                ui.spinner();
                return;
            };

            if !cache.logged_in {
                if ui.button("Link Spotify").clicked() {
                    self.send(SpotifyCommand::Login);
                }
                return;
            }

            self.render_currently_playing(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui
                    .checkbox(
                        &mut self.local_settings.pause_during_replay,
                        "Pause during replay",
                    )
                    .clicked()
                {
                    self.send_settings_update();
                }
            });

            if ui.button("Unlink").clicked() {
                self.send(SpotifyCommand::Logout);
            }
        })
        .response
    }
}
