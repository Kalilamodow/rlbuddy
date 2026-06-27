use crate::app::hotkey;
use crate::app::player_list::PlayerTable;
use crate::rl::{Platform, PlayerData, RLEvent, RankAPI, Team, TeamScores, connect_to_stats_api};
use eframe::egui::{self, Color32};
use std::cmp::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn systemtime_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

#[derive(Clone)]
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

pub struct RlBuddyApp {
    error_receiver: mpsc::Receiver<String>,
    current_error: Option<String>,

    rl_rx: mpsc::Receiver<RLEvent>,
    player_ranks: RankAPI,
    current_players: Option<Vec<MatchPlayer>>,
    current_score: Option<TeamScores>,

    prev_hide_pos: Option<egui::Pos2>,
    prev_match_info: Vec<MatchInfo>,

    overlay_tx: mpsc::Sender<bool>,
    overlay_rx: mpsc::Receiver<bool>,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (errors_tx, errors_rx) = mpsc::channel();
        let (overlay_tx, overlay_rx) = mpsc::channel();
        let (rl_tx, rl_rx) = mpsc::channel();

        let app = RlBuddyApp {
            current_players: None,
            current_score: None,
            rl_rx,
            error_receiver: errors_rx,
            player_ranks: RankAPI::new(cc.egui_ctx.clone(), errors_tx.clone()),
            current_error: None,
            overlay_tx: overlay_tx.clone(),
            overlay_rx,
            prev_hide_pos: None,
            prev_match_info: Vec::new(),
        };

        let overlay_tx_for_hotkey = overlay_tx.clone();
        let ctx_for_hotkey = ctx.clone();
        thread::spawn(move || {
            hotkey::listen_for_hotkey(overlay_tx_for_hotkey, ctx_for_hotkey);
        });

        thread::spawn(move || {
            let result = connect_to_stats_api(|event| {
                rl_tx.send(event).unwrap();
                ctx.request_repaint();
            });

            if let Err(error) = result {
                errors_tx.send(error.to_string()).unwrap();
            }
        });

        app
    }

    fn render_main_content(&self, ui: &mut egui::Ui) {
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
                    let winner_label = match prev_match.score.blue.cmp(&prev_match.score.orange) {
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
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.prev_hide_pos = ctx.input(|i| {
            i.viewport()
                .outer_rect
                .map(|outer_rect| egui::pos2(outer_rect.left(), outer_rect.top()))
        });

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(8.0, 8.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
    }

    fn hide(&self, ctx: &egui::Context) {
        if let Some(move_to) = self.prev_hide_pos {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(move_to));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnBottom,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
    }

    fn popup(&self) {
        let overlay_tx = self.overlay_tx.clone();
        // use here just for consistency
        overlay_tx.send(true).unwrap();

        thread::spawn(move || {
            thread::sleep(Duration::from_secs(3));
            overlay_tx.send(false).unwrap();
        });
    }
}

impl eframe::App for RlBuddyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(new_error) = self.error_receiver.try_recv() {
            self.current_error = Some(new_error);
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(err) = &self.current_error {
                ui.label(bold_text("Fatal error"));
                ui.label(err);
                if ui.button("Exit").clicked() {
                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
                self.render_main_content(ui);
            }
        });
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(should_overlay) = self.overlay_rx.try_recv() {
            if should_overlay {
                self.show(ctx);
            } else {
                self.hide(ctx);
            }
        }

        if let Ok(event) = self.rl_rx.try_recv() {
            match event {
                RLEvent::SetPlayerList(mut new_players) => {
                    let Some(players) = self.current_players.as_mut() else {
                        self.current_players =
                            Some(new_players.into_iter().map(Into::into).collect());
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
