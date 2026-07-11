use crate::common::ReadonlyStateHandle;

use super::{super::matches::MatchesServiceState, match_renderer::MatchRenderer};
use eframe::egui;

pub struct CurrentMatchWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
}

impl CurrentMatchWidget {
    pub fn new(state: ReadonlyStateHandle<MatchesServiceState>) -> Self {
        CurrentMatchWidget { state }
    }
}

impl egui::Widget for &CurrentMatchWidget {
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
                        ui.add(MatchRenderer::new(current_match));
                    }
                }
            } else {
                ui.label("Not in a match");
            }
        })
        .response
    }
}
