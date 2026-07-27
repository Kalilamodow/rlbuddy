use super::trn::{PlayerKey, TrackerAPI, new_tracker_api};
use crate::{
    player_info::trn_widget::TrackerWidget, rocket_league::Platform, stats_api::PlayerData,
};
use eframe::egui;

pub enum PlayerInfoServiceCommand {
    Open(Platform, String),
    OpenPlayer(PlayerData),
}

#[derive(Debug)]
struct OpenedPlayer {
    data: PlayerKey,
    open: bool,
}

pub struct PlayerInfoService {
    trn: TrackerAPI,
    open_players: Vec<OpenedPlayer>,
}

impl PlayerInfoService {
    pub fn new(context: egui::Context) -> Self {
        Self {
            trn: new_tracker_api(context),
            open_players: Vec::new(),
        }
    }

    pub fn update(&mut self, command: Option<PlayerInfoServiceCommand>) {
        let Some(command) = command else {
            return;
        };

        match command {
            PlayerInfoServiceCommand::OpenPlayer(p) => self.open_players.push(OpenedPlayer {
                data: p.into(),
                open: true,
            }),
            PlayerInfoServiceCommand::Open(platform, platform_id) => {
                self.open_players.push(OpenedPlayer {
                    data: PlayerKey {
                        platform,
                        platform_id,
                    },
                    open: true,
                })
            }
        }
    }
}

impl egui::Widget for &mut PlayerInfoService {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        for player in &mut self.open_players {
            let OpenedPlayer { open, data } = player;
            let profile = self.trn.get(data);

            egui::Window::new(
                profile
                    .as_ref()
                    .map_or(&data.platform_id, |p| &p.platform_info.platform_user_handle),
            )
            .open(open)
            .default_width(ui.available_width())
            .show(ui.ctx(), |ui| {
                if let Some(profile) = profile {
                    ui.add(TrackerWidget::new(profile));
                } else {
                    ui.spinner();
                }
            });
        }

        self.open_players.retain(|w| w.open);
        ui.allocate_response(egui::Vec2::ZERO, egui::Sense::empty())
    }
}
