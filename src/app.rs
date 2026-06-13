use crate::rl_stats_api;
use eframe::egui;
use std::sync::mpsc;
use std::thread;

fn bold_text(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong()
}

struct PlayerRanks {
    player_name: String,
    ranked_1s: String,
    ranked_2s: String,
    ranked_3s: String,
}

pub struct RankDisplayApp {
    players_receiver: mpsc::Receiver<Result<Vec<PlayerRanks>, String>>,
    players: Option<Vec<PlayerRanks>>,
    current_error: Option<String>,
}

impl RankDisplayApp {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let app = RankDisplayApp {
            players: None,
            players_receiver: rx,
            current_error: None,
        };

        thread::spawn(move || {
            let result = rl_stats_api::connect_to_stats_api(|player_datas| {
                tx.send(Ok(player_datas
                    .into_iter()
                    .map(|data| PlayerRanks {
                        player_name: data.name,
                        // TODO: get actual ranks here
                        ranked_1s: String::from("ssl"),
                        ranked_2s: String::from("ssl"),
                        ranked_3s: String::from("ssl"),
                    })
                    .collect()))
                    .unwrap();
            });

            if let Err(error) = result {
                tx.send(Err(error.to_string())).unwrap();
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
                        ui.label(&player.player_name);
                        ui.label(&player.ranked_1s);
                        ui.label(&player.ranked_2s);
                        ui.label(&player.ranked_3s);
                        ui.end_row();
                    }
                });
        } else {
            ui.label("connecting...");
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
