use super::rpc::{PresenceData, RichPresence};
use crate::rocket_league::{MatchState, Playlist, RLEvent};
use serde::{Deserialize, Serialize};
use std::{rc::Rc, sync::Arc};

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiscordSettings {
    pub disable: bool,
    pub hide_score: bool,
}

pub struct DiscordServiceState {
    pub settings: Rc<DiscordSettings>,
}

pub enum DiscordCommand {
    UpdateSettings(DiscordSettings),
}

pub struct DiscordService {
    settings: Rc<DiscordSettings>,
    rpc: RichPresence,
    current: GameState,
}

impl DiscordService {
    pub fn new(settings: Option<DiscordSettings>) -> Self {
        DiscordService {
            settings: Rc::new(settings.unwrap_or_default()),
            rpc: RichPresence::new(),
            current: GameState::Lobby,
        }
    }

    pub fn update(
        &mut self,
        stats_api_event: &Arc<Option<RLEvent>>,
        command: Option<DiscordCommand>,
    ) -> DiscordServiceState {
        if let Some(event) = stats_api_event.as_ref() {
            match event {
                RLEvent::Update(update) => {
                    self.current = match update.players.len() {
                        0 => GameState::Lobby,
                        1 => GameState::Training,
                        player_count => GameState::InGame(MatchData {
                            // TODO: actual scores
                            team_score: update.score.blue,
                            opp_score: update.score.orange,
                            playlist: Playlist::from_player_count(player_count),
                            arena: update.arena,
                            state: update.state.clone(),
                        }),
                    };
                }
                RLEvent::MatchLeft => {
                    self.current = GameState::Lobby;
                }
                _ => {}
            }
        }

        if let Some(command) = command {
            match command {
                DiscordCommand::UpdateSettings(new_settings) => {
                    self.settings = Rc::new(new_settings)
                }
            }
        }

        self.send_current();

        DiscordServiceState {
            settings: Rc::clone(&self.settings),
        }
    }

    fn send_current(&mut self) {
        if self.settings.disable {
            self.rpc.disconnect();
            return;
        } else {
            self.rpc.connect();
        }

        let presence = self.current.to_presence(!self.settings.hide_score);
        self.rpc.send(presence);
    }

    pub fn clone_settings(&self) -> DiscordSettings {
        self.settings.as_ref().clone()
    }
}
