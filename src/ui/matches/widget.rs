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
    super::spotify::SpotifyCommand,
    core::{MatchInfo, MatchOverInfo, MatchPlayer},
    match_renderer::MatchRenderer,
};
use crate::{
    discord,
    rl::{
        MatchUpdate, NameAPI, Platform, PlayerData, Playlist, RLEvent, RankAPI, Team,
        connect_to_stats_api,
    },
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

pub struct CurrentMatch {
    rl_rx: mpsc::Receiver<RLEvent>,
    player_ranks: RankAPI,
    player_names: NameAPI,
    rpc: Rc<Mutex<discord::RichPresence>>,
    match_data: Option<MatchInfo>,
    overlay_tx: mpsc::Sender<bool>,
    connected_to_rl: bool,
    spotify: mpsc::Sender<SpotifyCommand>,
    match_over_tx: mpsc::Sender<MatchInfo>,
}

impl CurrentMatch {
    pub fn new(
        ctx: &egui::Context,
        discord: Rc<Mutex<discord::RichPresence>>,
        overlay_tx: mpsc::Sender<bool>,
        spotify: mpsc::Sender<SpotifyCommand>,
        match_over_tx: mpsc::Sender<MatchInfo>,
    ) -> CurrentMatch {
        let (rl_tx, rl_rx) = mpsc::channel();

        let ctx_for_statsapi = ctx.clone();
        thread::spawn(move || {
            connect_to_stats_api(|event| {
                rl_tx.send(event).unwrap();
                ctx_for_statsapi.request_repaint();
            });
        });

        CurrentMatch {
            rl_rx,
            rpc: discord,
            player_ranks: RankAPI::new(ctx.clone()),
            player_names: NameAPI::new(ctx.clone()),
            match_data: None,
            overlay_tx,
            connected_to_rl: false,
            spotify,
            match_over_tx,
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

    fn update_state(&mut self, state: MatchUpdate) {
        if self.match_data.is_none() {
            self.match_data = Some(MatchInfo::default());
        }

        let Some(current_match) = self.match_data.as_mut() else {
            return;
        };

        current_match.max_active_players =
            current_match.max_active_players.max(state.players.len());
        current_match.score = state.score;
        diff_player_list(&mut current_match.players, state.players);

        for player in &mut current_match.players {
            if player.data.platform != Platform::Bot {
                player.skill = self.player_ranks.get(&player.data.platform_id);
            }

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
                    playlist: Playlist::from_player_count(current_match.max_active_players),
                    arena: state.arena,
                }));
        }
    }

    pub fn logic(&mut self, _ctx: &egui::Context) {
        if let Ok(event) = self.rl_rx.try_recv() {
            match event {
                RLEvent::MatchStart => {
                    self.match_data = Some(MatchInfo::default());
                    self.popup();
                }
                RLEvent::MatchOver(winner) => {
                    if let Some(current_match) = self.match_data.as_mut() {
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
                }
                RLEvent::Update(state) => {
                    self.update_state(state);
                }
                RLEvent::Connected => {
                    self.connected_to_rl = true;
                }
                RLEvent::Disconnected => {
                    self.connected_to_rl = false;
                }
                RLEvent::ReplayStart => {
                    self.spotify.send(SpotifyCommand::Pause).unwrap();
                }
                RLEvent::ReplayDone => {
                    self.spotify.send(SpotifyCommand::Play).unwrap();
                }
            }
        }
    }

    pub fn is_connected_to_rl(&self) -> bool {
        self.connected_to_rl
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
