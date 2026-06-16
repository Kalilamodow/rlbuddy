use crate::ranks::{Rank, RankAPI};
use crate::rl_stats_api::{self, PlayerData};
use eframe::egui;
use std::sync::mpsc;
use std::thread;

fn bold_text(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong()
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
            Rank::SSL => egui::include_image!("../assets/Supersonic_Legend_rank_icon.png"),
        }
    }
}

pub struct RankDisplayApp {
    players_receiver: mpsc::Receiver<Vec<PlayerData>>,
    error_receiver: mpsc::Receiver<String>,
    players: Option<Vec<PlayerData>>,
    player_ranks: RankAPI,
    current_error: Option<String>,
}

impl RankDisplayApp {
    pub fn new(ctx: &eframe::CreationContext) -> Self {
        let (player_tx, player_rx) = mpsc::channel();
        let (errors_tx, errors_rx) = mpsc::channel();

        let app = RankDisplayApp {
            players: None,
            players_receiver: player_rx,
            error_receiver: errors_rx,
            player_ranks: RankAPI::new(ctx.egui_ctx.clone(), errors_tx.clone()),
            current_error: None,
        };

        let ctx = ctx.egui_ctx.clone();
        thread::spawn(move || {
            let result = rl_stats_api::connect_to_stats_api(|player_datas| {
                if let Err(error) = player_tx.send(player_datas) {
                    eprintln!("[player_tx] error: {}", error);
                } else {
                    ctx.request_repaint();
                }
            });

            if let Err(error) = result {
                errors_tx.send(error.to_string()).unwrap();
            }
        });

        app
    }

    fn render_main_content(&mut self, ui: &mut egui::Ui) {
        if let Some(players) = &self.players {
            egui::Grid::new("player list")
                .num_columns(5)
                .spacing([12.0, 12.0])
                .striped(true)
                .min_row_height(32.0)
                .show(ui, |ui| {
                    ui.label(bold_text("Name"));
                    ui.label(bold_text("Platform"));
                    ui.label(bold_text("1s"));
                    ui.label(bold_text("2s"));
                    ui.label(bold_text("3s"));
                    ui.end_row();

                    for player in players {
                        ui.label(&player.name);
                        ui.label(player.platform.to_string());

                        if let Some(player_skills) = self.player_ranks.get(&player) {
                            let modes = [
                                &player_skills.ranked_1s,
                                &player_skills.ranked_2s,
                                &player_skills.ranked_3s,
                            ];

                            for skill in modes {
                                if let Some(skill) = skill {
                                    ui.image(skill.rank.to_image()).on_hover_text(format!(
                                        "{}\nMMR: {}",
                                        skill.rank.as_str(),
                                        skill.mmr
                                    ));
                                } else {
                                    ui.image(Rank::Unranked.to_image())
                                        .on_hover_text("No data for gamemode");
                                }
                            }
                        } else {
                            ui.spinner();
                            ui.spinner();
                            ui.spinner();
                        }
                        ui.end_row();
                    }
                });
        } else {
            ui.label("Waiting for game...");
            ui.spinner();
        }
    }
}

impl eframe::App for RankDisplayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(new_error) = self.error_receiver.try_recv() {
            self.current_error = Some(new_error);
        }
        if let Ok(new_players) = self.players_receiver.try_recv() {
            self.players = Some(new_players);
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Lobby info");
            ui.add_space(8.0);
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
}
