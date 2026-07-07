use std::sync::mpsc;

use super::{core::MatchInfo, match_renderer::MatchRenderer};
use eframe::egui;

pub struct PastMatchesWidget {
    matches: Vec<MatchInfo>,
    past_matches_tx: mpsc::Sender<MatchInfo>,
    past_matches_rx: mpsc::Receiver<MatchInfo>,
}

impl PastMatchesWidget {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        PastMatchesWidget {
            matches: Vec::new(),
            past_matches_tx: tx,
            past_matches_rx: rx,
        }
    }

    pub fn cmd(&self) -> mpsc::Sender<MatchInfo> {
        self.past_matches_tx.clone()
    }
}

impl egui::Widget for &mut PastMatchesWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        while let Ok(add_match) = self.past_matches_rx.try_recv() {
            self.matches.insert(0, add_match);
        }

        ui.vertical(|ui| {
            ui.add_space(4.0);
            for prev_match in &self.matches {
                ui.add(egui::Separator::default().spacing(8.0));
                ui.add(MatchRenderer::new(prev_match));
            }
        })
        .response
    }
}
