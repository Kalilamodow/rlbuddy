use super::rpc::{PresenceData, RichPresence};
use crate::{
    common::{ReadWriteStateHandle, ReadonlyStateHandle},
    rocket_league::{MatchState, MatchesServiceState, Playlist, Team},
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

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

pub struct DiscordService {
    settings: Rc<RefCell<DiscordSettings>>,
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
            settings: Rc::new(RefCell::new(settings.unwrap_or_default())),
            rpc: RichPresence::new(),
            current: GameState::Lobby,
            matches_handle,
        }
    }

    pub fn update(&mut self) {
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

        self.send_current();
    }

    fn send_current(&mut self) {
        let settings = self.settings.borrow();

        if settings.disable {
            self.rpc.ensure_disconnected();
            return;
        }
        self.rpc.ensure_connected();

        let presence = self.current.to_presence(!settings.hide_score);
        self.rpc.send(presence);
    }

    pub fn settings_handle(&self) -> ReadWriteStateHandle<DiscordSettings> {
        ReadWriteStateHandle::over(&self.settings)
    }
}
