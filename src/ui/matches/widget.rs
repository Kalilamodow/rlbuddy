// Displays the current and past matches

use std::{
    sync::{Arc, mpsc},
    time::SystemTime,
};

use eframe::egui;

use super::{
    core::{MatchInfo, MatchOverInfo, MatchPlayer},
    match_renderer::MatchRenderer,
};
use crate::rocket_league::{MatchUpdate, NameAPI, Platform, RLEvent, RankAPI, Team};

fn is_censored(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '*')
}

pub struct CurrentMatch {
    player_ranks: RankAPI,
    player_names: NameAPI,
    match_data: Option<MatchInfo>,
    match_over_tx: mpsc::Sender<MatchInfo>,
    local_player_id: Option<String>,
}

impl CurrentMatch {
    pub fn new(ctx: &egui::Context, match_over_tx: mpsc::Sender<MatchInfo>) -> CurrentMatch {
        CurrentMatch {
            player_ranks: RankAPI::new(ctx.clone()),
            player_names: NameAPI::new(ctx.clone()),
            match_data: None,
            match_over_tx,
            local_player_id: None,
        }
    }

    fn update_state(&mut self, mut updated: MatchUpdate) {
        if self.match_data.is_none() {
            self.match_data = Some(MatchInfo::default());
        }

        let Some(current_match) = self.match_data.as_mut() else {
            return;
        };

        current_match.max_active_players =
            current_match.max_active_players.max(updated.players.len());
        current_match.score = updated.score;

        // bots all share the same id so replace it for comparisons
        for player_or_bot_hmm in &mut updated.players {
            if player_or_bot_hmm.platform == Platform::Bot {
                player_or_bot_hmm.platform_id = player_or_bot_hmm.name.clone();
            }
        }

        for player in &mut current_match.players {
            let updated_pos = updated
                .players
                .iter()
                .position(|p| p.platform_id == player.data.platform_id);
            if let Some(updated_pos) = updated_pos {
                let updated = updated.players.swap_remove(updated_pos);
                player.data = updated;
                player.left = false;
            } else {
                player.left = true;
            }
        }

        for remaining_player in updated.players {
            current_match.players.push(MatchPlayer {
                is_local_player: Some(&remaining_player.name) == self.local_player_id.as_ref(),
                left: false,
                uncensored_name: None,
                skill: None,
                data: remaining_player,
            });
        }

        for player in &mut current_match.players {
            if player.data.platform != Platform::Bot {
                player.skill = self.player_ranks.get(&player.data.platform_id);
            }

            if is_censored(player.display_name()) {
                player.uncensored_name = self.player_names.get(&player.data.platform_id);
            }
        }

        current_match.our_team = current_match
            .players
            .iter()
            .find(|p| p.is_local_player)
            .map_or(Team::Blue, |p| p.data.team);

        current_match
            .players
            .sort_by_key(|p| p.data.team != current_match.our_team);
    }

    pub fn logic(&mut self, ctx: &egui::Context, stats_api_event: &Arc<Option<RLEvent>>) {
        if let Some(event) = stats_api_event.as_ref() {
            match event {
                RLEvent::MatchStart => {
                    self.match_data = Some(MatchInfo::default());
                    ctx.request_repaint();
                }
                RLEvent::MatchOver(winner) => {
                    if let Some(current_match) = self.match_data.as_mut() {
                        if current_match.players.len() <= 1 {
                            return;
                        }

                        current_match.finish = Some(MatchOverInfo {
                            timestamp: SystemTime::now(),
                            winner: Some(*winner),
                        });
                    }
                    ctx.request_repaint();
                }
                RLEvent::MatchLeft => {
                    let Some(mut current_match) = self.match_data.take() else {
                        return;
                    };

                    // training
                    if current_match.players.len() <= 1 {
                        self.match_data = Some(current_match);
                        return;
                    }

                    if current_match.finish.is_none() {
                        current_match.finish = Some(MatchOverInfo {
                            timestamp: SystemTime::now(),
                            winner: current_match.score.guess_winner(),
                        });
                    }

                    self.match_over_tx.send(current_match).unwrap();
                    ctx.request_repaint();
                }
                RLEvent::Update(state) => {
                    self.update_state(state.clone());
                    ctx.request_repaint();
                }
                RLEvent::OurPlayerId(id) => {
                    self.local_player_id = Some(id.clone());
                }
                _ => {}
            }
        }
    }
}

impl egui::Widget for &CurrentMatch {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if let Some(current_match) = &self.match_data {
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
