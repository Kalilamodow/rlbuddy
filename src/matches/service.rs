use std::sync::Arc;
use std::time::SystemTime;

use eframe::egui;

use crate::common::{ReadWriteStateHandle, ReadonlyStateHandle};
use crate::matches::models::{MatchInfo, MatchOverInfo, MatchPlayer};
use crate::stats_api::{MatchUpdate, Platform, RLEvent, Team};

use super::apis::{NameAPI, RankAPI};

#[derive(Debug, Default)]
pub struct MatchesServiceState {
    pub current_match: Option<MatchInfo>,
    pub prev_matches: Vec<MatchInfo>,
}

pub struct MatchesService {
    state: ReadWriteStateHandle<MatchesServiceState>,

    local_player_id: Option<String>,
    rank_api: RankAPI,
    names_api: NameAPI,
}

impl MatchesService {
    pub fn new(ctx: &egui::Context) -> Self {
        MatchesService {
            state: ReadWriteStateHandle::default(),
            local_player_id: None,
            rank_api: RankAPI::new(ctx.clone()),
            names_api: NameAPI::new(ctx.clone()),
        }
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<MatchesServiceState> {
        ReadonlyStateHandle::over(&self.state)
    }

    fn update_state(&mut self, mut updated: MatchUpdate) {
        let mut state = self.state.write();

        if state.current_match.is_none() {
            state.current_match = Some(MatchInfo::default());
        }

        let Some(current_match) = state.current_match.as_mut() else {
            return;
        };

        current_match.arena = Some(updated.arena);
        current_match.state = updated.state;
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
                is_local_player: Some(&remaining_player.platform_id)
                    == self.local_player_id.as_ref(),
                left: false,
                uncensored_name: None,
                skill: None,
                data: remaining_player,
            });
        }

        for player in &mut current_match.players {
            if player.data.platform != Platform::Bot {
                player.skill = self.rank_api.get(&player.data.platform_id);
            }

            if is_censored(player.display_name()) {
                player.uncensored_name = self.names_api.get(&player.data.platform_id);
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

    pub fn update(&mut self, ctx: &egui::Context, stats_api_event: &Arc<Option<RLEvent>>) {
        let Some(event) = stats_api_event.as_ref() else {
            return;
        };

        let mut state = self.state.write();
        match event {
            RLEvent::MatchStart => {
                state.current_match = Some(MatchInfo::default());
                ctx.request_repaint();
            }
            RLEvent::MatchOver(winner) => {
                if let Some(current_match) = state.current_match.as_mut() {
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
                let Some(mut current_match) = state.current_match.take() else {
                    return;
                };

                if current_match.players.len() <= 1 {
                    return;
                }

                if current_match.finish.is_none() {
                    current_match.finish = Some(MatchOverInfo {
                        timestamp: SystemTime::now(),
                        winner: current_match.score.guess_winner(),
                    });
                }

                state.prev_matches.push(current_match);
                ctx.request_repaint();
            }
            RLEvent::Update(update) => {
                drop(state);
                self.update_state(update.clone());
                ctx.request_repaint();
            }
            RLEvent::OurPlayerId(id) => {
                self.local_player_id = Some(id.clone());
            }
            _ => {}
        }
    }
}

fn is_censored(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '*')
}
