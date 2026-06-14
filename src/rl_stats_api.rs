use serde::Deserialize;
use std::{
    fmt,
    io::Read,
    net::{SocketAddr, TcpStream},
    str::FromStr,
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

#[derive(Debug)]
pub enum Platform {
    Epic,
    Steam,
    Xbox,
}

#[derive(Debug)]
pub struct UnknownPlatform;

impl FromStr for Platform {
    type Err = UnknownPlatform;
    fn from_str(s: &str) -> Result<Platform, Self::Err> {
        match s {
            "Epic" => Ok(Platform::Epic),
            "Steam" => Ok(Platform::Steam),
            "XboxOne" => Ok(Platform::Xbox),
            _ => Err(UnknownPlatform),
        }
    }
}

pub struct PlayerData {
    pub name: String,
    pub platform: Platform,
    pub platform_id: String,
}

fn parse_stats_api_player(value: StatsApiPlayerData) -> Option<PlayerData> {
    let parts: Vec<&str> = value.id_data.split("|").collect();

    if let Ok(platform) = Platform::from_str(parts[0]) {
        Some(PlayerData {
            name: value.name,
            platform: platform,
            platform_id: String::from(parts[1]),
        })
    } else {
        None
    }
}

impl fmt::Display for PlayerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) [{}]",
            self.name,
            match self.platform {
                Platform::Epic => "epic",
                Platform::Steam => "steam",
                Platform::Xbox => "xbox",
            },
            self.platform_id
        )
    }
}

pub enum StatsApiError {
    CouldNotConnect,
    InvalidStatsApiMessage(String),
}

impl fmt::Display for StatsApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CouldNotConnect => write!(
                f,
                "couldnt connect to statsapi (make sure you have it enabled)"
            ),
            Self::InvalidStatsApiMessage(s) => write!(f, "got an invalid stats api message: {s}"),
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
            Ok(0) => continue,
            Ok(b) => b,
            Err(_) => return Err(StatsApiError::CouldNotConnect),
        };

        let text = match std::str::from_utf8(&read_buffer[..n_bytes]) {
            Ok(t) => t,
            Err(_) => {
                return Err(StatsApiError::InvalidStatsApiMessage(String::from(
                    "cant decode",
                )));
            }
        };

        let Ok(event) = serde_json::from_str::<StatsApiEvent>(&text) else {
            // ignore (probably framing issue)
            continue;
        };

        if event.event == "UpdateState" {
            let data: UpdateStateEventData = serde_json::from_str(&event.data)
                .map_err(|e| StatsApiError::InvalidStatsApiMessage(e.to_string() + text))?;
            on_player_update(
                data.players
                    .into_iter()
                    .map(parse_stats_api_player)
                    .filter_map(std::convert::identity)
                    .collect(),
            );
        };
    }
}
