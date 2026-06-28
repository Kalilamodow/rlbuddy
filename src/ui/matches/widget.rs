// Displays the current and past matches

use std::{
    sync::mpsc,
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;

use super::{
    core::{MatchInfo, MatchOverInfo, MatchPlayer},
    match_renderer::MatchRenderer,
};
use crate::rl::{Platform, PlayerData, RLEvent, RankAPI, Team, connect_to_stats_api};

fn diff_player_list(current: &mut Vec<MatchPlayer>, mut new_players: Vec<PlayerData>) {
    // bots all share the same id so replace it for comparisons
    for player_or_bot_hmm in &mut new_players {
        if player_or_bot_hmm.platform == Platform::Bot {
            player_or_bot_hmm.platform_id = player_or_bot_hmm.name.clone();
        }
    }

    for player in current.iter_mut() {
        let updated_pos = new_players
            .iter()
            .position(|p| p.platform_id == player.data.platform_id);
        if let Some(updated_pos) = updated_pos {
            let updated = new_players.swap_remove(updated_pos);
            player.data = updated;
            player.left = false;
        } else {
            player.left = true;
        }
    }

    for remaining_to_add in new_players {
        current.push(remaining_to_add.into());
    }
}

pub struct Matches {
    rl_rx: mpsc::Receiver<RLEvent>,
    player_ranks: RankAPI,
    current_match: Option<MatchInfo>,
    prev_match_info: Vec<MatchInfo>,
    overlay_tx: mpsc::Sender<bool>,
}

impl Matches {
    pub fn new(
        ctx: egui::Context,
        overlay_tx: mpsc::Sender<bool>,
        errors_tx: mpsc::Sender<String>,
    ) -> Matches {
        let (rl_tx, rl_rx) = mpsc::channel();

        let ctx_for_statsapi = ctx.clone();
        let errors_tx_for_statsapi = errors_tx.clone();
        thread::spawn(move || {
            let result = connect_to_stats_api(|event| {
                rl_tx.send(event).unwrap();
                ctx_for_statsapi.request_repaint();
            });

            if let Err(error) = result {
                errors_tx_for_statsapi.send(error.to_string()).unwrap();
            }
        });

        Matches {
            rl_rx,
            player_ranks: RankAPI::new(ctx, errors_tx),
            current_match: None,
            prev_match_info: Vec::new(),
            overlay_tx,
        }
    }

    fn popup(&self) {
        let overlay_tx = self.overlay_tx.clone();
        overlay_tx.send(true).unwrap();

        thread::spawn(move || {
            thread::sleep(Duration::from_secs(3));
            overlay_tx.send(false).unwrap();
        });
    }

    pub fn logic(&mut self, _ctx: &egui::Context) {
        if let Ok(event) = self.rl_rx.try_recv() {
            match event {
                RLEvent::MatchStart => {
                    self.current_match = Some(MatchInfo::default());
                    self.popup();
                }
                RLEvent::MatchOver(winner) => {
                    if let Some(current_match) = self.current_match.as_mut() {
                        if current_match.players.len() <= 1 {
                            return;
                        }

                        current_match.finish = Some(MatchOverInfo {
                            timestamp: SystemTime::now(),
                            winner: Some(winner),
                        });
                    }
                }
                RLEvent::MatchLeft => {
                    if self
                        .current_match
                        .as_ref()
                        .is_none_or(|m| m.players.len() <= 1)
                    {
                        return;
                    }

                    self.prev_match_info
                        .insert(0, self.current_match.take().unwrap());
                }
                RLEvent::Update(state) => {
                    if self.current_match.is_none() {
                        self.current_match = Some(MatchInfo::default());
                    }

                    let Some(current_match) = self.current_match.as_mut() else {
                        return;
                    };

                    current_match.score = state.score;
                    diff_player_list(&mut current_match.players, state.players);

                    current_match.our_team = current_match
                        .players
                        .iter()
                        .find(|p| p.data.is_self)
                        .map_or(Team::Blue, |p| p.data.team);

                    current_match
                        .players
                        .sort_by_key(|p| p.data.team != current_match.our_team);
                }
            }
        }
    }
}

impl egui::Widget for &Matches {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if let Some(current_match) = &self.current_match {
                match current_match.players.len() {
                    0 => {
                        ui.label("No players");
                    }
                    1 => {
                        ui.label("In freeplay");
                    }
                    _ => {
                        ui.add(MatchRenderer::new(current_match, &self.player_ranks));
                    }
                }
            } else {
                ui.label("Not in a match");
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for prev_match in &self.prev_match_info {
                    ui.add(egui::Separator::default().spacing(8.0));
                    ui.add(MatchRenderer::new(prev_match, &self.player_ranks));
                }
            });
        })
        .response
    }
}
