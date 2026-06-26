use crate::hotkey;
use crate::ranks::{EventRanks, Rank, RankAPI};
use crate::rl_stats_api::{self, Platform, PlayerData, RLEvent, Team};
use eframe::egui::{self, Color32};
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{fmt, thread};

fn systemtime_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

#[derive(Clone)]
struct MatchPlayer {
    left: bool,
    data: PlayerData,
}

impl From<PlayerData> for MatchPlayer {
    fn from(value: PlayerData) -> Self {
        MatchPlayer {
            left: false,
            data: value,
        }
    }
}

fn sort_player_list(players: &mut Vec<MatchPlayer>) {
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

#[derive(PartialEq)]
enum Playlist {
    Freeplay,
    Ones,
    Twos,
    Threes,
    Other,
}

impl Playlist {
    fn from_player_count(player_count: usize) -> Playlist {
        match player_count {
            1 => Playlist::Freeplay,
            2 => Playlist::Ones,
            4 => Playlist::Twos,
            6 => Playlist::Threes,
            _ => Playlist::Other,
        }
    }
}

impl fmt::Display for Playlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Playlist::Ones => "1s",
                Playlist::Twos => "2s",
                Playlist::Threes => "3s",
                Playlist::Freeplay => "Freeplay",
                Playlist::Other => "Some",
            }
        )
    }
}

fn center_layout<R>(
    ui: &mut egui::Ui,
    height: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        add_contents,
    )
}

fn center_label(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
) -> egui::InnerResponse<egui::Response> {
    center_layout(ui, 16.0, |ui| ui.label(text))
}

pub struct RlBuddyApp {
    error_receiver: mpsc::Receiver<String>,
    current_error: Option<String>,

    rl_rx: mpsc::Receiver<RLEvent>,
    current_players: Option<Vec<MatchPlayer>>,
    player_ranks: RankAPI,

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
                    ui.label(bold_text(format!("{}", prev_match.winner)));
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
                            Some(new_players.into_iter().map(|p| p.into()).collect());
                        return;
                    };

                    // bots all share the same id so replace it for comparisons
                    for player_or_bot_hmm in new_players.iter_mut() {
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

struct PlayerTable<'a> {
    players: &'a Vec<MatchPlayer>,
    id: &'a str,
    ranks: &'a RankAPI,
}

impl<'a> PlayerTable<'a> {
    fn new(players: &'a Vec<MatchPlayer>, id: &'a str, ranks: &'a RankAPI) -> PlayerTable<'a> {
        PlayerTable { players, id, ranks }
    }

    fn render_player(&self, ui: &mut egui::Ui, playlist: &Playlist, match_player: &MatchPlayer) {
        let skill = if match_player.data.platform == Platform::Bot {
            None
        } else {
            self.ranks.get(&match_player.data.platform_id)
        };

        // rank in this gamemode
        if let Some(skill) = &skill {
            PlayerTable::render_player_rank_cell(ui, playlist, skill);
        } else {
            center_label(ui, "-");
        }

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;

            let name_color = if match_player.left {
                Color32::GRAY
            } else {
                match match_player.data.team {
                    Team::Blue => Color32::from_rgb(64, 128, 255),
                    Team::Orange => Color32::ORANGE,
                }
            };
            ui.add(
                egui::Label::new(
                    bold_text(&match_player.data.name)
                        .color(name_color)
                        .size(15.0),
                )
                .extend(),
            );

            if let Some(skill) = &skill {
                PlayerTable::render_rank_list(ui, match_player.left, skill);
            }
        });

        center_label(ui, match_player.data.score.to_string());
        ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
        ui.end_row();
    }

    fn render_player_rank_cell(ui: &mut egui::Ui, playlist: &Playlist, skill: &Arc<EventRanks>) {
        let rank = match playlist {
            Playlist::Ones => skill.duels.as_ref(),
            Playlist::Twos => skill.doubles.as_ref(),
            Playlist::Threes => skill.standard.as_ref(),
            _ => None,
        };

        match rank {
            Some(rank) => {
                center_layout(ui, 28.0, |ui| {
                    if rank.rank_is_estimate {
                        ui.add(
                            egui::Image::new(Rank::Unranked.to_image())
                                .fit_to_exact_size(egui::vec2(28.0, 28.0)),
                        )
                        .on_hover_text(format!("Unranked in {}", playlist))
                    } else {
                        ui.add(
                            egui::Image::new(rank.rank.to_image())
                                .fit_to_exact_size(egui::vec2(28.0, 28.0)),
                        )
                        .on_hover_text(format!(
                            "{} rank: {}{}",
                            playlist,
                            rank.rank.as_str(),
                            rank.div
                        ))
                    }
                });
            }
            None => {
                center_label(ui, "-");
            }
        }
    }

    fn render_rank_list(ui: &mut egui::Ui, muted: bool, skill: &Arc<EventRanks>) {
        ui.horizontal(|ui| {
            let modes = [&skill.duels, &skill.doubles, &skill.standard];

            for mode in modes {
                // per-rank mmr + icon
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    if let Some(mode) = mode {
                        let image = ui.image(mode.rank.to_image());
                        if mode.rank_is_estimate {
                            image.on_hover_text("Estimated rank");
                        } else {
                            image.on_hover_text(
                                mode.rank.as_str().to_string() + &mode.div.to_string(),
                            );
                        }

                        if muted {
                            ui.label(mode.mmr.to_string());
                        } else {
                            ui.label(
                                egui::RichText::new(mode.mmr.to_string())
                                    .color(mode.rank.to_color()),
                            );
                        }
                    } else {
                        ui.image(Rank::Unranked.to_image());
                        ui.label(egui::RichText::new("---").color(Rank::Unranked.to_color()));
                    }
                });
            }
        });
    }
}

impl egui::Widget for PlayerTable<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let playlist = Playlist::from_player_count(self.players.iter().filter(|p| !p.left).count());

        // 3 columns + allocate_space hack
        // https://github.com/emilk/egui/issues/3928
        egui::Grid::new(self.id)
            .spacing(egui::vec2(8.0, 12.0))
            .striped(true)
            .num_columns(3)
            .show(ui, |ui| {
                center_label(ui, bold_text("Rank"));
                ui.label(bold_text("Player"));
                center_label(ui, bold_text("Score"));

                ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                ui.end_row();

                for player in self.players {
                    self.render_player(ui, &playlist, player);
                }
            })
            .response
    }
}
