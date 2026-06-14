use crate::ranks::PlayerRankInformation;
use crate::rl_stats_api::{self, PlayerData};
use eframe::egui;
use std::sync::mpsc;
use std::thread;

fn bold_text(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong()
}

pub struct RankDisplayApp {
    players_receiver: mpsc::Receiver<Result<Vec<PlayerData>, String>>,
    players: Option<Vec<PlayerData>>,
    player_ranks: PlayerRankInformation,
    current_error: Option<String>,
}

impl RankDisplayApp {
    pub fn new(ctx: &eframe::CreationContext) -> Self {
        let (player_tx, player_rx) = mpsc::channel();
        let app = RankDisplayApp {
            players: None,
            players_receiver: player_rx,
            player_ranks: PlayerRankInformation::new(ctx.egui_ctx.clone()),
            current_error: None,
        };

        let ctx = ctx.egui_ctx.clone();
        thread::spawn(move || {
            let result = rl_stats_api::connect_to_stats_api(|player_datas| {
                player_tx.send(Ok(player_datas)).unwrap();
                ctx.request_repaint();
            });

            if let Err(error) = result {
                player_tx.send(Err(error.to_string())).unwrap();
            }
        });

        app
    }

    fn render_main_content(&mut self, ui: &mut egui::Ui) {
        if let Some(players) = &self.players {
            egui::Grid::new("player list")
                .num_columns(4)
                .spacing([12.0, 12.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(bold_text("Name"));
                    ui.label(bold_text("1s"));
                    ui.label(bold_text("2s"));
                    ui.label(bold_text("3s"));
                    ui.end_row();

                    for player in players {
                        ui.label(&player.name);

                        if let Some(ranks) = self.player_ranks.get(&player) {
                            ui.label(match &ranks.ranked_1s {
                                Some(txt) => txt,
                                None => "None",
                            });
                            ui.label(match &ranks.ranked_2s {
                                Some(txt) => txt,
                                None => "None",
                            });
                            ui.label(match &ranks.ranked_3s {
                                Some(txt) => txt,
                                None => "None",
                            });
                        } else {
                            ui.label("Loading...");
                            ui.label("Loading...");
                            ui.label("Loading...");
                        }
                        ui.end_row();
                    }
                });
        } else {
            ui.label("waiting for game...");
            ui.spinner();
        }
    }
}

impl eframe::App for RankDisplayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(message) = self.players_receiver.try_recv() {
            match message {
                Ok(new_players) => self.players = Some(new_players),
                Err(error) => self.current_error = Some(error),
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("player ranks");
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
