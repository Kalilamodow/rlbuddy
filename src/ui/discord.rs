use std::sync::mpsc::{self, Sender};
use std::thread;

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::discord::{PresenceData, RichPresence};
use crate::rocket_league::{MatchState, Playlist};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RichPresenceSettings {
    disable: bool,
    hide_score: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchData {
    pub team_score: u8,
    pub opp_score: u8,
    pub playlist: Option<Playlist>,
    pub arena: &'static str,
    pub state: MatchState,
}

impl MatchData {
    pub fn generate_presence(&self, include_score: bool) -> PresenceData {
        let mut details = format!(
            "{} in {}",
            self.playlist
                .as_ref()
                .map(ToString::to_string)
                .as_deref()
                .unwrap_or("Playing"),
            self.arena
        );

        if include_score {
            details += format!(" | {}-{}", self.team_score, self.opp_score).as_str();
        }

        PresenceData {
            details,
            state: Some(self.state.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Lobby,
    Training,
    InGame(MatchData),
}

impl GameState {
    fn to_presence(&self, show_score: bool) -> PresenceData {
        match &self {
            GameState::Lobby => PresenceData {
                details: "Main menu".to_owned(),
                state: None,
            },
            GameState::Training => PresenceData {
                details: "In training".to_owned(),
                state: None,
            },
            GameState::InGame(game) => game.generate_presence(show_score),
        }
    }
}

#[derive(Debug)]
pub enum DiscordCommand {
    UpdateState(GameState),
    Connect,
    Disconnect,
    EnableScores,
    DisableScores,
}

struct PresenceController {
    rpc: RichPresence,
    current: GameState,
    scores_enabled: bool,
}

impl PresenceController {
    pub fn new() -> Self {
        PresenceController {
            rpc: RichPresence::new(),
            current: GameState::Lobby,
            scores_enabled: true,
        }
    }

    pub fn mainloop(&mut self, rx: mpsc::Receiver<DiscordCommand>) {
        loop {
            while let Ok(cmd) = rx.try_recv() {
                let mut should_update = true;

                match cmd {
                    DiscordCommand::Connect => {
                        self.rpc.connect();
                    }
                    DiscordCommand::Disconnect => {
                        self.rpc.disconnect();
                        should_update = false;
                    }
                    DiscordCommand::EnableScores => {
                        self.scores_enabled = true;
                    }
                    DiscordCommand::DisableScores => {
                        self.scores_enabled = false;
                    }
                    DiscordCommand::UpdateState(state) => {
                        self.current = state;
                    }
                }

                if should_update {
                    self.rpc.send(self.current.to_presence(self.scores_enabled));
                }
            }
        }
    }
}

pub struct DiscordWidget {
    settings: RichPresenceSettings,
    controller: Sender<DiscordCommand>,
}

impl DiscordWidget {
    pub fn new(settings: Option<RichPresenceSettings>) -> DiscordWidget {
        let settings = settings.unwrap_or_default();

        let (controller, mainloop_receiver) = mpsc::channel();
        thread::spawn(move || {
            PresenceController::new().mainloop(mainloop_receiver);
        });

        if !settings.disable {
            controller.send(DiscordCommand::Connect).unwrap();
        }

        if settings.hide_score {
            controller.send(DiscordCommand::DisableScores).unwrap();
        } else {
            controller.send(DiscordCommand::EnableScores).unwrap();
        }

        DiscordWidget {
            settings,
            controller,
        }
    }

    pub fn clone_settings(&self) -> RichPresenceSettings {
        self.settings.clone()
    }

    pub fn cmd(&self) -> mpsc::Sender<DiscordCommand> {
        self.controller.clone()
    }
}

impl egui::Widget for &mut DiscordWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            if ui
                .checkbox(&mut self.settings.disable, "Disabled")
                .changed()
            {
                if self.settings.disable {
                    self.controller.send(DiscordCommand::Disconnect).unwrap();
                } else {
                    self.controller.send(DiscordCommand::Connect).unwrap();
                }
            }

            ui.add_enabled_ui(!self.settings.disable, |ui| {
                if ui
                    .checkbox(&mut self.settings.hide_score, "Hide score")
                    .changed()
                {
                    if self.settings.hide_score {
                        self.controller.send(DiscordCommand::DisableScores).unwrap();
                    } else {
                        self.controller.send(DiscordCommand::EnableScores).unwrap();
                    }
                }
            })
        })
        .response
    }
}
