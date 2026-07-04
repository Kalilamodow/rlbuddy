use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Timestamps},
};

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
pub struct Scores {
    pub blue: u8,
    pub orange: u8,
    pub winning: WinState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Lobby,
    Training,
    InGame(Scores),
}

pub struct RichPresence {
    previous_send: Option<State>,
    client: DiscordIpcClient,
    start_time: i64,
}

const APP_ID: &str = "356877880938070016";

impl RichPresence {
    pub fn new() -> RichPresence {
        let mut client = DiscordIpcClient::new(APP_ID);
        client.connect().unwrap();

        RichPresence {
            previous_send: None,
            client,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .try_into()
                .unwrap(),
        }
    }

    pub fn set(&mut self, state: State) {
        if self.previous_send.as_ref() == Some(&state) {
            return;
        }

        self.client
            .set_activity(
                Activity::new()
                    .details(match &state {
                        State::Lobby => format!("Main menu"),
                        State::Training => format!("In training"),
                        State::InGame(score) => {
                            format!("{} {}-{}", score.winning, score.blue, score.orange)
                        }
                    })
                    .timestamps(Timestamps::new().start(self.start_time)),
            )
            .unwrap();

        self.previous_send = Some(state);
    }
}
