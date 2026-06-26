use crate::hotkey;
use crate::ranks::{Rank, RankAPI};
use crate::rl_stats_api::{self, Platform, PlayerData, RLEvent, Team};
use eframe::egui::{self, Color32};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn systemtime_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

struct MatchInfo {
    pub players: Vec<PlayerData>,
    pub timestamp: SystemTime,
    pub winner: Team,
}

impl Rank {
    pub fn to_image(&self) -> egui::ImageSource<'static> {
        match self {
            Rank::Unranked => egui::include_image!("../assets/Unranked_icon.png"),
            Rank::Bronze1 => egui::include_image!("../assets/Bronze1_rank_icon.png"),
            Rank::Bronze2 => egui::include_image!("../assets/Bronze2_rank_icon.png"),
            Rank::Bronze3 => egui::include_image!("../assets/Bronze3_rank_icon.png"),
            Rank::Silver1 => egui::include_image!("../assets/Silver1_rank_icon.png"),
            Rank::Silver2 => egui::include_image!("../assets/Silver2_rank_icon.png"),
            Rank::Silver3 => egui::include_image!("../assets/Silver3_rank_icon.png"),
            Rank::Gold1 => egui::include_image!("../assets/Gold1_rank_icon.png"),
            Rank::Gold2 => egui::include_image!("../assets/Gold2_rank_icon.png"),
            Rank::Gold3 => egui::include_image!("../assets/Gold3_rank_icon.png"),
            Rank::Plat1 => egui::include_image!("../assets/Platinum1_rank_icon.png"),
            Rank::Plat2 => egui::include_image!("../assets/Platinum2_rank_icon.png"),
            Rank::Plat3 => egui::include_image!("../assets/Platinum3_rank_icon.png"),
            Rank::Diamond1 => egui::include_image!("../assets/Diamond1_rank_icon.png"),
            Rank::Diamond2 => egui::include_image!("../assets/Diamond2_rank_icon.png"),
            Rank::Diamond3 => egui::include_image!("../assets/Diamond3_rank_icon.png"),
            Rank::Champ1 => egui::include_image!("../assets/Champion1_rank_icon.png"),
            Rank::Champ2 => egui::include_image!("../assets/Champion2_rank_icon.png"),
            Rank::Champ3 => egui::include_image!("../assets/Champion3_rank_icon.png"),
            Rank::GC1 => egui::include_image!("../assets/Grand_Champion1_rank_icon.png"),
            Rank::GC2 => egui::include_image!("../assets/Grand_Champion2_rank_icon.png"),
            Rank::GC3 => egui::include_image!("../assets/Grand_Champion3_rank_icon.png"),
            Rank::Ssl => egui::include_image!("../assets/Supersonic_Legend_rank_icon.png"),
        }
    }

    pub fn to_color(&self) -> Color32 {
        match self {
            Rank::Unranked => Color32::DARK_GRAY,
            Rank::Bronze1 | Rank::Bronze2 | Rank::Bronze3 => Color32::BROWN,
            Rank::Silver1 | Rank::Silver2 | Rank::Silver3 => Color32::GRAY,
            Rank::Gold1 | Rank::Gold2 | Rank::Gold3 => Color32::YELLOW,
            Rank::Plat1 | Rank::Plat2 | Rank::Plat3 => Color32::LIGHT_BLUE,
            Rank::Diamond1 | Rank::Diamond2 | Rank::Diamond3 => Color32::BLUE,
            Rank::Champ1 | Rank::Champ2 | Rank::Champ3 => Color32::PURPLE,
            Rank::GC1 | Rank::GC2 | Rank::GC3 => Color32::RED,
            Rank::Ssl => Color32::WHITE,
        }
    }
}

pub struct RankDisplayApp {
    error_receiver: mpsc::Receiver<String>,
    current_error: Option<String>,

    rl_rx: mpsc::Receiver<RLEvent>,
    current_players: Option<Vec<PlayerData>>,
    player_ranks: RankAPI,

    prev_hide_pos: Option<egui::Pos2>,
    prev_match_info: Vec<MatchInfo>,

    overlay_tx: mpsc::Sender<bool>,
    overlay_rx: mpsc::Receiver<bool>,
}

impl RankDisplayApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (errors_tx, errors_rx) = mpsc::channel();
        let (overlay_tx, overlay_rx) = mpsc::channel();
        let (rl_tx, rl_rx) = mpsc::channel();

        let app = RankDisplayApp {
            current_players: None,
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
        thread::spawn(move || {
            hotkey::listen_for_hotkey(overlay_tx_for_hotkey);
        });

        thread::spawn(move || {
            let result = rl_stats_api::connect_to_stats_api(|event| {
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(current_players) = &self.current_players {
                ui.label("Current match");

                if current_players.is_empty() {
                    ui.label("No players");
                } else {
                    self.render_players(ui, current_players, "current", true);
                }
            }

            let current_time = SystemTime::now();
            for prev_match in &self.prev_match_info {
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(bold_text(format!("{}", prev_match.winner)));
                    ui.label(format!(
                        "{} seconds ago",
                        current_time
                            .duration_since(prev_match.timestamp)
                            .unwrap_or_default()
                            .as_secs()
                    ));
                });

                self.render_players(
                    ui,
                    &prev_match.players,
                    systemtime_since_epoch(prev_match.timestamp)
                        .to_string()
                        .as_str(),
                    false,
                );
            }
        });
    }

    fn render_players(&self, ui: &mut egui::Ui, players: &Vec<PlayerData>, id: &str, main: bool) {
        // 3 columns + allocate_space hack
        // https://github.com/emilk/egui/issues/3928
        egui::Grid::new(id)
            .spacing(egui::vec2(16.0, 8.0))
            .striped(true)
            .num_columns(3)
            .show(ui, |ui| {
                if main {
                    ui.label(bold_text("Player"));
                    ui.label(bold_text("Score"));
                } else {
                    ui.label("Player");
                    ui.label("Score");
                }
                ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                ui.end_row();

                for player in players {
                    let skill = if player.platform == Platform::Bot {
                        None
                    } else {
                        self.player_ranks.get(&player.platform_id)
                    };

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;

                        ui.label(
                            bold_text(&player.name)
                                .color(match player.team {
                                    Team::Blue => Color32::from_rgb(64, 128, 255),
                                    Team::Orange => Color32::ORANGE,
                                })
                                .size(15.0),
                        );

                        ui.horizontal(|ui| {
                            if let Some(skill) = skill.clone() {
                                let modes = [&skill.duels, &skill.doubles, &skill.standard];

                                for mode in modes {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;

                                        if let Some(mode) = mode {
                                            let image = ui.image(mode.rank.to_image());
                                            if mode.rank_is_estimate {
                                                image.on_hover_text("Estimated rank");
                                            } else {
                                                image.on_hover_text(
                                                    mode.rank.as_str().to_string()
                                                        + &mode.div.to_string(),
                                                );
                                            }

                                            ui.label(
                                                egui::RichText::new(mode.mmr.to_string())
                                                    .color(mode.rank.to_color()),
                                            );
                                        } else {
                                            ui.label("None");
                                        }
                                    });
                                }
                            }
                        });
                    });
                    ui.label(player.score.to_string());
                    ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                    ui.end_row();
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

impl eframe::App for RankDisplayApp {
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
                    // group by team
                    let our_team = new_players
                        .iter()
                        .find(|p| p.is_self)
                        .map_or(Team::Blue, |p| p.team);
                    // != bc false comes first
                    new_players.sort_by_key(|p| p.team != our_team);

                    self.current_players = Some(new_players);
                }
                RLEvent::MatchStart => {
                    self.popup();
                }
                RLEvent::MatchEnd(team) => {
                    if let Some(players) = &self.current_players {
                        self.prev_match_info.insert(
                            0,
                            MatchInfo {
                                players: players.clone(),
                                timestamp: SystemTime::now(),
                                winner: team,
                            },
                        );

                        self.current_players = Some(Vec::new());
                    }
                }
            }
        }
    }
}
