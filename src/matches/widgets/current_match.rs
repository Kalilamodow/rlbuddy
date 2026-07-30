use crate::{
    common::ReadonlyStateHandle, matches::models::MatchPlayer,
    player_info::PlayerInfoServiceCommand,
};

use super::{super::service::MatchesServiceState, match_renderer::MatchRenderer};
use eframe::egui;

pub struct CurrentMatchWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
    wants_more_player_info: Option<MatchPlayer>,
}

impl CurrentMatchWidget {
    pub fn new(state: ReadonlyStateHandle<MatchesServiceState>) -> Self {
        CurrentMatchWidget {
            state,
            wants_more_player_info: None,
        }
    }

    pub fn get_command(&mut self) -> Option<PlayerInfoServiceCommand> {
        self.wants_more_player_info
            .take()
            .map(|i| PlayerInfoServiceCommand::OpenPlayer(i.data))
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
                            &mut self.wants_more_player_info,
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
