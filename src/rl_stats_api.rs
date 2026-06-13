use serde::Deserialize;
use std::{
    fmt,
    io::Read,
    net::{SocketAddr, TcpStream},
};

fn or_error<R, OldE, NewE>(r: Result<R, OldE>, e: NewE) -> Result<R, NewE> {
    r.map_err(|_| e)
}

#[derive(Debug, Deserialize)]
struct StatsApiEvent {
    #[serde(rename = "Event")]
    event: String,
    /// data is a json string
    #[serde(rename = "Data")]
    data: String,
}

#[derive(Debug, Deserialize)]
struct StatsApiPlayerData {
    #[serde(rename = "Name")]
    name: String,
    /// "Platform identifier in the format Platform|Uid|Splitscreen (e.g. "Steam|123|0", "Epic|456|0")."
    #[serde(rename = "PrimaryId")]
    id_data: String,
    // theres other stuff but we can just ignore it
}

#[derive(Debug, Deserialize)]
struct UpdateStateEventData {
    #[serde(rename = "Players")]
    players: Vec<StatsApiPlayerData>,
}

pub struct PlayerData {
    pub name: String,
    pub platform: String,
    pub uuid: String,
}

impl From<StatsApiPlayerData> for PlayerData {
    fn from(value: StatsApiPlayerData) -> Self {
        let parts: Vec<&str> = value.id_data.split("|").collect();

        PlayerData {
            name: value.name,
            platform: String::from(parts[0]),
            uuid: String::from(parts[1]),
        }
    }
}

pub enum StatsApiError {
    CouldNotConnect,
    Disconnected,
    InvalidStatsApiMessage,
}

impl fmt::Display for StatsApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CouldNotConnect => write!(
                f,
                "couldnt connect to statsapi (make sure you have it enabled)"
            ),
            Self::Disconnected => write!(f, "disconnected from rl"),
            Self::InvalidStatsApiMessage => write!(f, "got an invalid stats api message"),
        }
    }
}

pub fn connect_to_stats_api<F: Fn(Vec<PlayerData>)>(
    on_player_update: F,
) -> Result<(), StatsApiError> {
    let mut read_buffer = vec![0u8; 4096];

    let mut tcp = or_error(
        TcpStream::connect(&"127.0.0.1:49123".parse::<SocketAddr>().unwrap()),
        StatsApiError::CouldNotConnect,
    )?;

    loop {
        let n_bytes = match tcp.read(&mut read_buffer) {
            Ok(0) => return Err(StatsApiError::Disconnected),
            Ok(b) => b,
            Err(_) => return Err(StatsApiError::CouldNotConnect),
        };

        let text = match std::str::from_utf8(&read_buffer[..n_bytes]) {
            Ok(t) => t,
            Err(_) => return Err(StatsApiError::InvalidStatsApiMessage),
        };

        let event: StatsApiEvent = or_error(
            serde_json::from_str(&text),
            StatsApiError::InvalidStatsApiMessage,
        )?;

        if event.event == "UpdateState" {
            let data: UpdateStateEventData = or_error(
                serde_json::from_str(&event.data),
                StatsApiError::InvalidStatsApiMessage,
            )?;
            on_player_update(data.players.into_iter().map(Into::into).collect());
        };
    }
}
