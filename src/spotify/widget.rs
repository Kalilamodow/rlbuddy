use super::service::{
    SPOTIFY_REFRESH_INTERVAL, SpotifyCommand, SpotifyServiceState, SpotifySettings,
};
use crate::common::{ReadWriteStateHandle, ThreadedReadonlyStateHandle};
use eframe::egui::{self, TextBuffer};
use std::time::SystemTime;

pub struct SpotifyWidget {
    state: ThreadedReadonlyStateHandle<SpotifyServiceState>,
    settings: ReadWriteStateHandle<SpotifySettings>,
    send_command: Option<SpotifyCommand>,
    is_setting_up: bool,
    inputted_client_id: String,
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
            is_setting_up: false,
            inputted_client_id: String::new(),
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

                if self.state.read().is_updating {
                    ui.spinner();
                }
            });
        });
    }

    fn render_setup(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Setup instructions");
            ui.small("Note: needs spotify premium");
        });

        ui.horizontal_wrapped(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;

            ui.label("Step 1: go to the ");
            if ui.link("Spotify developer dashboard").clicked() {
                let _ = webbrowser::open("https://developer.spotify.com/dashboard");
            }
        });

        ui.label("Step 2: click \"Create app\" and fill in the app name/description with whatever (eg. rlbuddy)");
        ui.horizontal_wrapped(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;

            ui.label("Step 3: add this redirect uri: ");
            ui.code("http://127.0.0.1:7742/");
        });

        ui.label("Step 4: check the \"Web API\" checkbox");
        ui.label("Step 5: agree with the tos and press Save");

        ui.add_space(4.0);
        ui.label("Now, copy the Client ID and paste it in here:");

        ui.text_edit_singleline(&mut self.inputted_client_id);
        ui.add_space(4.0);

        ui.label("Once you've put in the client id, click the button to log in.");

        let has_entered_client_id = !self.inputted_client_id.is_empty();
        if ui
            .add_enabled(has_entered_client_id, egui::Button::new("Log in"))
            .clicked()
        {
            self.is_setting_up = false;
            let client_id = self.inputted_client_id.take();
            self.send(SpotifyCommand::Login(client_id));
        }

        ui.add_space(8.0);
        if ui.small_button("Cancel setup").clicked() {
            self.is_setting_up = false;
        }
    }

    fn send(&mut self, cmd: SpotifyCommand) {
        self.send_command = Some(cmd);
    }
}

impl egui::Widget for &mut SpotifyWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if self.is_setting_up {
                self.render_setup(ui);
                return;
            }

            {
                let state = self.state.read();

                if !state.logged_in {
                    drop(state);

                    if ui.button("Set up Spotify").clicked() {
                        self.is_setting_up = true;
                    }
                    return;
                }

                if state.is_updating {
                    ui.disable();
                }
            }

            self.render_currently_playing(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let mut settings = self.settings.write();
                ui.checkbox(&mut settings.pause_during_replay, "Pause during anthems");
            });

            if ui.button("Unlink").clicked() {
                self.send(SpotifyCommand::Logout);
            }
        })
        .response
    }
}
