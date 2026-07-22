use super::{super::service::MatchesServiceState, match_renderer::MatchRenderer};
use crate::common::ReadonlyStateHandle;
use eframe::egui;
use std::{collections::HashMap, time::SystemTime};

pub struct PastMatchesWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
    open: HashMap<SystemTime, bool>,
}

impl PastMatchesWidget {
    pub fn new(state: ReadonlyStateHandle<MatchesServiceState>) -> Self {
        PastMatchesWidget {
            state,
            open: HashMap::new(),
        }
    }
}

impl egui::Widget for &mut PastMatchesWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.vertical(|ui| {
                for prev_match in self.state.read().prev_matches.iter().rev() {
                    ui.group(|ui| {
                        ui.add(MatchRenderer::new(
                            prev_match,
                            Some(&mut self.open.entry(prev_match.started_at).or_insert(false)),
                        ))
                    });
                }
            })
            .response
        })
        .response
    }
}
