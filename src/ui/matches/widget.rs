// Displays the current and past matches

use std::{
    cmp::Ordering,
    rc::Rc,
    sync::{Mutex, mpsc},
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;

use super::{
    core::{MatchInfo, MatchOverInfo, MatchPlayer},
    match_renderer::MatchRenderer,
};
use crate::{
    discord,
    rl::{NameAPI, Platform, PlayerData, Playlist, RLEvent, RankAPI, Team, connect_to_stats_api},
};

fn is_censored(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '*')
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

pub struct Matches {
    rl_rx: mpsc::Receiver<RLEvent>,
    player_ranks: RankAPI,
    player_names: NameAPI,
    rpc: Rc<Mutex<discord::RichPresence>>,
    current_match: Option<MatchInfo>,
    prev_match_info: Vec<MatchInfo>,
    overlay_tx: mpsc::Sender<bool>,
    connected_to_rl: bool,
}

impl Matches {
    pub fn new(
        ctx: &egui::Context,
        discord: Rc<Mutex<discord::RichPresence>>,
        overlay_tx: mpsc::Sender<bool>,
        errors_tx: mpsc::Sender<String>,
    ) -> Matches {
        let (rl_tx, rl_rx) = mpsc::channel();

        let ctx_for_statsapi = ctx.clone();
        thread::spawn(move || {
            connect_to_stats_api(|event| {
                rl_tx.send(event).unwrap();
                ctx_for_statsapi.request_repaint();
            });
        });

        Matches {
            rl_rx,
            rpc: discord,
            player_ranks: RankAPI::new(ctx.clone(), errors_tx),
            player_names: NameAPI::new(ctx.clone()),
            current_match: None,
            prev_match_info: Vec::new(),
            overlay_tx,
            connected_to_rl: false,
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
                    self.rpc.lock().unwrap().set(discord::State::Lobby);

                    let Some(mut current_match) = self.current_match.take() else {
                        return;
                    };

                    // training
                    if current_match.players.len() <= 1 {
                        self.current_match = Some(current_match);
                        return;
                    }

                    if current_match.finish.is_none() {
                        current_match.finish = Some(MatchOverInfo {
                            timestamp: SystemTime::now(),
                            winner: current_match.score.guess_winner(),
                        });
                    }

                    self.prev_match_info.insert(0, current_match);
                }
                RLEvent::Update(state) => {
                    if self.current_match.is_none() {
                        self.current_match = Some(MatchInfo::default());
                    }

                    let Some(current_match) = self.current_match.as_mut() else {
                        return;
                    };

                    current_match.max_active_players =
                        current_match.max_active_players.max(state.players.len());
                    current_match.score = state.score;
                    diff_player_list(&mut current_match.players, state.players);

                    for player in &mut current_match.players {
                        if is_censored(&player.data.name) {
                            player.uncensor_with(&self.player_names);
                        }
                    }

                    current_match.our_team = current_match
                        .players
                        .iter()
                        .find(|p| p.data.is_self)
                        .map_or(Team::Blue, |p| p.data.team);

                    current_match
                        .players
                        .sort_by_key(|p| p.data.team != current_match.our_team);

                    if current_match.players.len() == 1 {
                        self.rpc.lock().unwrap().set(discord::State::Training);
                    } else {
                        let (our, theirs) = match current_match.our_team {
                            Team::Blue => (current_match.score.blue, current_match.score.orange),
                            Team::Orange => (current_match.score.orange, current_match.score.blue),
                        };
                        let winning = match our.cmp(&theirs) {
                            Ordering::Greater => discord::WinState::Winning,
                            Ordering::Less => discord::WinState::Losing,
                            Ordering::Equal => discord::WinState::Tied,
                        };

                        self.rpc
                            .lock()
                            .unwrap()
                            .set(discord::State::InGame(discord::GameData {
                                blue: current_match.score.blue,
                                orange: current_match.score.orange,
                                winning,
                                playlist: Playlist::from_player_count(
                                    current_match.max_active_players,
                                ),
                                arena: state.arena,
                            }));
                    }
                }
                RLEvent::Connected => {
                    self.connected_to_rl = true;
                }
                RLEvent::Disconnected => {
                    self.connected_to_rl = false;
                }
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected_to_rl
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
