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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Platform {
    Epic,
    Steam,
    Xbox,
    PlayStation,
    Switch,
    Bot,
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
            "PS4" => Ok(Platform::PlayStation),
            "Switch" => Ok(Platform::Switch),
            "Unknown" => Ok(Platform::Bot),
            _ => Err(UnknownPlatform),
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Platform::Epic => "Epic",
                Platform::Steam => "Steam",
                Platform::PlayStation => "PlayStation",
                Platform::Xbox => "Xbox",
                Platform::Switch => "Switch",
                Platform::Bot => "Bot",
            }
        )
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
            platform_id: value.id_data,
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
            self.name, self.platform, self.platform_id
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

pub enum RLEvent {
    SetPlayerList(Vec<PlayerData>),
    MatchStart,
}

pub fn connect_to_stats_api<F: Fn(RLEvent)>(on_event: F) -> Result<(), StatsApiError> {
    let mut read_buffer = vec![0u8; 4096];

    let mut tcp = or_error(
        TcpStream::connect(&"127.0.0.1:49123".parse::<SocketAddr>().unwrap()),
        StatsApiError::CouldNotConnect,
    )?;
    on_event(RLEvent::SetPlayerList(Vec::new()));

    // MatchInitialized doesnt fire in private matches for some reason
    // so listen for match created then the first countdown is the "game start"
    let mut match_created_event_happened = false;

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

        match event.event.as_str() {
            "UpdateState" => {
                let data: UpdateStateEventData = serde_json::from_str(&event.data)
                    .map_err(|e| StatsApiError::InvalidStatsApiMessage(e.to_string() + text))?;
                on_event(RLEvent::SetPlayerList(
                    data.players
                        .into_iter()
                        .map(parse_stats_api_player)
                        .filter_map(std::convert::identity)
                        .collect(),
                ));
            }
            "MatchCreated" => {
                match_created_event_happened = true;
            }
            "CountdownBegin" => {
                if match_created_event_happened {
                    match_created_event_happened = false;
                    on_event(RLEvent::MatchStart);
                }
            }
            _ => {}
        }
    }
}
