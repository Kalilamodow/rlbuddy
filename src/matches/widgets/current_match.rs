use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    player_info::PlayerInfoServiceCommand,
};

use super::{super::service::MatchesServiceState, match_renderer::MatchRenderer};
use eframe::egui;

pub struct CurrentMatchWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
    player_info_sender: Sender<PlayerInfoServiceCommand>,
}

impl CurrentMatchWidget {
    pub fn new(
        state: ReadonlyStateHandle<MatchesServiceState>,
        player_info_sender: Sender<PlayerInfoServiceCommand>,
    ) -> Self {
        CurrentMatchWidget {
            state,
            player_info_sender,
        }
    }
}

impl egui::Widget for &mut CurrentMatchWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if let Some(current_match) = &self.state.read().current_match {
                match current_match.players.len() {
                    0 => {
                        ui.label("No players");
                    }
                    1 => {
                        ui.label("In freeplay");
                    }
                    _ => {
                        ui.add(MatchRenderer::new(
                            current_match,
                            None,
                            &self.player_info_sender,
                        ));
                    }
                }
            } else {
                ui.label("Not in a match");
            }
        })
        .response
    }
}
