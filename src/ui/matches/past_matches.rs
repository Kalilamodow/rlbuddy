use std::sync::mpsc;

use eframe::egui;

use crate::rl::RankAPI;

use super::{core::MatchInfo, match_renderer::MatchRenderer};

pub struct PastMatchesWidget {
    matches: Vec<MatchInfo>,
    player_ranks: RankAPI,
    past_matches_tx: mpsc::Sender<MatchInfo>,
    past_matches_rx: mpsc::Receiver<MatchInfo>,
}

impl PastMatchesWidget {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let (error_tx, _) = mpsc::channel();

        PastMatchesWidget {
            matches: Vec::new(),
            player_ranks: RankAPI::new(ctx, error_tx),
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
                ui.add(MatchRenderer::new(prev_match, &self.player_ranks));
            }
        })
        .response
    }
}
