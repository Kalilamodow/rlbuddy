// Displays the current and past matches

use std::{
    cmp::Ordering,
    sync::mpsc,
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui::{self, Color32};

use crate::{
    app::player_list::PlayerTable,
    rl::{Platform, PlayerData, RLEvent, RankAPI, Team, TeamScores, connect_to_stats_api},
};

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

fn score_labels(ui: &mut egui::Ui, scores: &TeamScores, priority: Team) {
    let blue_text = egui::RichText::new(scores.blue.to_string()).color(Color32::LIGHT_BLUE);
    let orange_text = egui::RichText::new(scores.orange.to_string()).color(Color32::LIGHT_RED);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if priority == Team::Blue {
            ui.label(blue_text);
            ui.label("-");
            ui.label(orange_text);
        } else {
            ui.label(orange_text);
            ui.label("-");
            ui.label(blue_text);
        }
    });
}

fn pluralize_ago(count: u64, word: &str, suffix: &str) -> String {
    format!(
        "{count} {word}{} {suffix}",
        if count == 1 { "" } else { "s" }
    )
}

const ONE_SECOND: Duration = Duration::from_secs(1);
const ONE_MINUTE: Duration = Duration::from_mins(1);

pub fn format_seconds(seconds: u64) -> (String, Duration) {
    match seconds {
        ..60 => (pluralize_ago(seconds, "second", "ago"), ONE_SECOND),
        60..3600 => (pluralize_ago(seconds / 60, "minute", "ago"), ONE_MINUTE),
        3600.. => (
            format!(
                "{}{}",
                pluralize_ago(seconds / 3600, "hour", ""),
                pluralize_ago((seconds % 3600) / 60, "minute", "ago")
            ),
            ONE_MINUTE,
        ),
    }
}

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

#[derive(Debug, Clone)]
pub struct MatchPlayer {
    pub left: bool,
    pub data: PlayerData,
}

impl From<PlayerData> for MatchPlayer {
    fn from(value: PlayerData) -> Self {
        MatchPlayer {
            left: false,
            data: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchOverInfo {
    pub timestamp: SystemTime,
    pub winner: Option<Team>,
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub players: Vec<MatchPlayer>,
    pub score: TeamScores,
    pub our_team: Team,
    pub finish: Option<MatchOverInfo>,
    pub started_at: SystemTime,
}

impl Default for MatchInfo {
    fn default() -> Self {
        MatchInfo {
            players: Vec::new(),
            score: TeamScores { blue: 0, orange: 0 },
            our_team: Team::Blue,
            finish: None,
            started_at: SystemTime::now(),
        }
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
                    self.current_match = Some(Default::default());
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
                    };

                    self.prev_match_info
                        .insert(0, self.current_match.take().unwrap());
                }
                RLEvent::Update(state) => {
                    if self.current_match.is_none() {
                        self.current_match = Some(Default::default());
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
                        .map(|p| p.data.team)
                        .unwrap_or(Team::Blue);

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
                ui.horizontal(|ui| {
                    ui.label("Current match");
                    score_labels(ui, &current_match.score, current_match.our_team);
                });

                match current_match.players.len() {
                    0 => {
                        ui.label("No players");
                    }
                    1 => {
                        ui.label("In freeplay");
                    }
                    _ => {
                        ui.add(PlayerTable::new(current_match, &self.player_ranks, true));
                    }
                }
            } else {
                ui.label("Not in a match");
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                let current_time = SystemTime::now();
                for prev_match in &self.prev_match_info {
                    ui.add(egui::Separator::default().spacing(8.0));

                    ui.horizontal(|ui| {
                        if let Some(over) = &prev_match.finish {
                            let winner = match over.winner {
                                Some(winner) => Some(winner),
                                None => match prev_match.score.blue.cmp(&prev_match.score.orange) {
                                    Ordering::Greater => Some(Team::Blue),
                                    Ordering::Less => Some(Team::Orange),
                                    Ordering::Equal => None,
                                },
                            };

                            if let Some(winner) = winner {
                                if winner == prev_match.our_team {
                                    ui.label(bold_text("Win"));
                                } else {
                                    ui.label(bold_text("Loss"));
                                }
                            }

                            score_labels(ui, &prev_match.score, prev_match.our_team);

                            let seconds_ago = current_time
                                .duration_since(over.timestamp)
                                .unwrap_or_default()
                                .as_secs();

                            let (formatted_time, update_after) = format_seconds(seconds_ago);
                            ui.label(formatted_time);
                            ui.ctx().request_repaint_after(update_after);
                        };
                    });

                    ui.add(PlayerTable::new(prev_match, &self.player_ranks, false));
                }
            });
        })
        .response
    }
}
