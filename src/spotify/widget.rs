use super::service::{
    SPOTIFY_REFRESH_INTERVAL, SpotifyCommand, SpotifyServiceState, SpotifySettings,
};
use crate::common::{ReadWriteStateHandle, ThreadedReadonlyStateHandle};
use eframe::egui;
use std::time::SystemTime;

pub struct SpotifyWidget {
    state: ThreadedReadonlyStateHandle<SpotifyServiceState>,
    settings: ReadWriteStateHandle<SpotifySettings>,
    send_command: Option<SpotifyCommand>,
}

impl SpotifyWidget {
    pub fn new(
        state_handle: ThreadedReadonlyStateHandle<SpotifyServiceState>,
        settings_handle: ReadWriteStateHandle<SpotifySettings>,
    ) -> SpotifyWidget {
        SpotifyWidget {
            state: state_handle,
            settings: settings_handle,
            send_command: None,
        }
    }

    pub fn get_command(&mut self) -> Option<SpotifyCommand> {
        self.send_command.take()
    }

    fn render_time_till_next_poll(&self, ui: &mut egui::Ui) {
        let state = self.state.read();

        let seconds_since = SystemTime::now()
            .duration_since(state.last_updated_at)
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
                let state = self.state.read();
                let Some(currently_playing) = state.playback_state.as_ref() else {
                    has_song = false;
                    ui.label("No track currently playing");
                    return;
                };

                let track = &currently_playing.track;
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

    fn send(&mut self, cmd: SpotifyCommand) {
        self.send_command = Some(cmd);
    }
}

impl egui::Widget for &mut SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            {
                let state = self.state.read();

                if !state.logged_in {
                    drop(state);

                    if ui.button("Link Spotify").clicked() {
                        self.send(SpotifyCommand::Login);
                    }
                    return;
                }
            }

            self.render_currently_playing(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let mut settings = self.settings.write();
                ui.checkbox(&mut settings.pause_during_replay, "Pause during replay");
            });

            if ui.button("Unlink").clicked() {
                self.send(SpotifyCommand::Logout);
            }
        })
        .response
    }
}
