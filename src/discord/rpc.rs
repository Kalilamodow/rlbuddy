use std::time::{SystemTime, UNIX_EPOCH};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Timestamps},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceData {
    pub details: String,
    pub state: Option<String>,
}

pub struct RichPresence {
    previous_send: Option<PresenceData>,
    client: DiscordIpcClient,
    is_connected: bool,
    start_time: i64,
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
            is_connected: false,
        }
    }

    pub fn connect(&mut self) {
        if self.is_connected {
            return;
        }
        self.client.connect().unwrap();
    }

    pub fn disconnect(&mut self) {
        if !self.is_connected {
            return;
        }

        self.client.close().unwrap();
        self.previous_send = None;
    }

    pub fn send(&mut self, presence: PresenceData) {
        if Some(&presence) == self.previous_send.as_ref() {
            return;
        }

        let mut activity = Activity::new()
            .timestamps(Timestamps::new().start(self.start_time))
            .details(&presence.details);

        if let Some(state) = presence.state.as_ref() {
            activity = activity.state(state);
        }

        self.client.set_activity(activity).unwrap();
        self.previous_send = Some(presence);
    }
}
