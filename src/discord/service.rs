use super::rpc::{PresenceData, RichPresence};
use crate::{
    common::ReadonlyStateHandle,
    rocket_league::{MatchState, MatchesServiceState, Playlist, Team},
};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

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
    matches_handle: ReadonlyStateHandle<MatchesServiceState>,
}

impl DiscordService {
    pub fn new(
        settings: Option<DiscordSettings>,
        matches_handle: ReadonlyStateHandle<MatchesServiceState>,
    ) -> Self {
        DiscordService {
            settings: Rc::new(settings.unwrap_or_default()),
            rpc: RichPresence::new(),
            current: GameState::Lobby,
            matches_handle,
        }
    }

    pub fn update(&mut self, command: Option<DiscordCommand>) -> DiscordServiceState {
        self.current = match &self.matches_handle.read().current_match {
            Some(current_match) => match current_match.players.len() {
                0 => GameState::Lobby,
                1 => GameState::Training,
                player_count => {
                    let (our_score, their_score) = match current_match.our_team {
                        Team::Blue => (current_match.score.blue, current_match.score.orange),
                        Team::Orange => (current_match.score.orange, current_match.score.blue),
                    };

                    GameState::InGame(MatchData {
                        team_score: our_score,
                        opp_score: their_score,
                        playlist: Playlist::from_player_count(player_count),
                        arena: current_match.arena.unwrap_or_default(),
                        state: current_match.state.clone(),
                    })
                }
            },
            None => GameState::Lobby,
        };

        if let Some(command) = command {
            match command {
                DiscordCommand::UpdateSettings(new_settings) => {
                    self.settings = Rc::new(new_settings);
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
            self.rpc.ensure_disconnected();
            return;
        }
        self.rpc.ensure_connected();

        let presence = self.current.to_presence(!self.settings.hide_score);
        self.rpc.send(presence);
    }

    pub fn clone_settings(&self) -> DiscordSettings {
        self.settings.as_ref().clone()
    }
}
