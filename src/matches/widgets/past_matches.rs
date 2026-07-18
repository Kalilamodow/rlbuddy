use crate::common::ReadonlyStateHandle;

use super::{super::service::MatchesServiceState, match_renderer::MatchRenderer};
use eframe::egui;

pub struct PastMatchesWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
}

impl PastMatchesWidget {
    pub fn new(state: ReadonlyStateHandle<MatchesServiceState>) -> Self {
        PastMatchesWidget { state }
    }
}

impl egui::Widget for &PastMatchesWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.vertical(|ui| {
                ui.add_space(4.0);
                for prev_match in self.state.read().prev_matches.iter().rev() {
                    ui.add(egui::Separator::default().spacing(8.0));
                    ui.add(MatchRenderer::new(prev_match));
                }
            })
            .response
        })
        .response
    }
}
