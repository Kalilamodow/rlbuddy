use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Timestamps},
};

use crate::rl::Playlist;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WinState {
    Winning,
    Losing,
    Tied,
}

impl Display for WinState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WinState::Losing => "Losing",
                WinState::Winning => "Winning",
                WinState::Tied => "Tied",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameData {
    pub blue: u8,
    pub orange: u8,
    pub winning: WinState,
    pub playlist: Option<Playlist>,
    pub arena: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Lobby,
    Training,
    InGame(GameData),
}

pub struct RichPresence {
    previous_send: Option<State>,
    client: DiscordIpcClient,
    start_time: i64,
    should_show_score: bool,
}

const APP_ID: &str = "356877880938070016";

impl RichPresence {
    pub fn new() -> RichPresence {
        RichPresence {
            client: DiscordIpcClient::new(APP_ID),
            previous_send: None,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .try_into()
                .unwrap(),
            should_show_score: true,
        }
    }

    pub fn connect(&mut self) {
        self.client.connect().unwrap();
        if let Some(prev) = self.previous_send.take() {
            self.send(prev);
        } else {
            self.send(State::Lobby);
        }
    }

    pub fn disconnect(&mut self) {
        self.client.close().unwrap();
    }

    pub fn show_score(&mut self) {
        self.should_show_score = true;
        if let Some(prev) = self.previous_send.take() {
            self.send(prev);
        }
    }

    pub fn hide_score(&mut self) {
        self.should_show_score = false;
        if let Some(prev) = self.previous_send.take() {
            self.send(prev);
        }
    }

    pub fn set(&mut self, state: State) {
        if self.previous_send.as_ref() == Some(&state) {
            return;
        }

        self.send(state);
    }

    fn send(&mut self, state: State) {
        let activity = Activity::new().timestamps(Timestamps::new().start(self.start_time));
        let activity = match &state {
            State::Lobby => activity.details("Main menu"),
            State::Training => activity.details("In training"),
            State::InGame(data) => {
                let mut activity = activity.details(format!(
                    "{} in {}",
                    data.playlist
                        .as_ref()
                        .map(ToString::to_string)
                        .as_deref()
                        .unwrap_or("Playing"),
                    data.arena
                ));

                if self.should_show_score {
                    activity =
                        activity.state(format!("{} {}-{}", data.winning, data.blue, data.orange));
                }

                activity
            }
        };

        self.client.set_activity(activity).unwrap();
        self.previous_send = Some(state);
    }
}
