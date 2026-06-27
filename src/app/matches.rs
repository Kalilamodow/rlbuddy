// Displays the current and past matches

use std::{
    cmp::Ordering,
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use eframe::egui::{self, Color32};

use crate::{
    app::player_list::PlayerTable,
    rl::{Platform, PlayerData, RLEvent, RankAPI, Team, TeamScores, connect_to_stats_api},
};

fn systemtime_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
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

fn sort_player_list(players: &mut [MatchPlayer]) {
    // group by team
    let our_team = players
        .iter()
        .find(|p| p.data.is_self)
        .map_or(Team::Blue, |p| p.data.team);
    // != bc false comes first
    players.sort_by_key(|p| p.data.team != our_team);
}

struct MatchInfo {
    pub players: Vec<MatchPlayer>,
    pub timestamp: SystemTime,
    pub score: TeamScores,
}

pub struct Matches {
    rl_rx: mpsc::Receiver<RLEvent>,
    player_ranks: RankAPI,
    current_players: Option<Vec<MatchPlayer>>,
    current_score: Option<TeamScores>,
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
            current_players: None,
            current_score: None,
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

    fn diff_player_list(&mut self, mut new_players: Vec<PlayerData>) {
        let Some(players) = self.current_players.as_mut() else {
            self.current_players = Some(new_players.into_iter().map(Into::into).collect());
            return;
        };

        // bots all share the same id so replace it for comparisons
        for player_or_bot_hmm in &mut new_players {
            if player_or_bot_hmm.platform == Platform::Bot {
                player_or_bot_hmm.platform_id = player_or_bot_hmm.name.clone();
            }
        }

        for player in players.iter_mut() {
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
            players.push(remaining_to_add.into());
        }

        sort_player_list(players);
    }

    pub fn logic(&mut self, _ctx: &egui::Context) {
        if let Ok(event) = self.rl_rx.try_recv() {
            match event {
                RLEvent::SetPlayerList(new_players) => {
                    self.diff_player_list(new_players);
                }
                RLEvent::SetScore(score) => {
                    self.current_score = Some(score);
                }
                RLEvent::MatchStart => {
                    self.popup();
                }
                RLEvent::MatchEnd => {
                    if let Some(players) = &self.current_players {
                        if players.len() <= 1 {
                            return;
                        }

                        self.prev_match_info.insert(
                            0,
                            MatchInfo {
                                players: players.clone(),
                                timestamp: SystemTime::now(),
                                score: self.current_score.take().unwrap_or_default(),
                            },
                        );

                        self.current_players = None;
                    }
                }
            }
        }
    }
}

impl egui::Widget for &Matches {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if let Some(current_players) = &self.current_players {
                ui.label("Current match");
                match current_players.len() {
                    0 => {
                        ui.label("No players");
                    }
                    1 => {
                        ui.label("In freeplay");
                    }
                    _ => {
                        ui.add(PlayerTable::new(
                            current_players,
                            "current match",
                            &self.player_ranks,
                        ));
                    }
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                let current_time = SystemTime::now();
                for prev_match in &self.prev_match_info {
                    ui.add(egui::Separator::default().spacing(8.0));

                    ui.horizontal(|ui| {
                        let winner_label = match prev_match.score.blue.cmp(&prev_match.score.orange)
                        {
                            Ordering::Greater => "Blue won",
                            Ordering::Less => "Orange won",
                            Ordering::Equal => "Tie",
                        };

                        ui.label(bold_text(winner_label));

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(
                                egui::RichText::new(prev_match.score.blue.to_string())
                                    .color(Color32::LIGHT_BLUE),
                            );
                            ui.label("-");
                            ui.label(
                                egui::RichText::new(prev_match.score.orange.to_string())
                                    .color(Color32::LIGHT_RED),
                            );
                        });

                        ui.label(format!(
                            "{} seconds ago",
                            current_time
                                .duration_since(prev_match.timestamp)
                                .unwrap_or_default()
                                .as_secs()
                        ));
                    });

                    ui.add(PlayerTable::new(
                        &prev_match.players,
                        systemtime_since_epoch(prev_match.timestamp)
                            .to_string()
                            .as_str(),
                        &self.player_ranks,
                    ));
                }
            });
        })
        .response
    }
}
