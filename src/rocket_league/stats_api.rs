use serde::Deserialize;
use std::{
    cmp::Ordering,
    fmt,
    io::Read,
    net::{SocketAddr, TcpStream},
    str::FromStr,
    time::Duration,
};

use crate::rocket_league::MatchState;

#[derive(Debug, Deserialize)]
struct StatsApiEvent {
    #[serde(rename = "Event")]
    event: String,
    /// data is a json string
    #[serde(rename = "Data")]
    data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiPlayerData {
    name: String,
    /// "Platform identifier in the format Platform|Uid|Splitscreen (e.g. "Steam|123|0", "Epic|456|0")."
    primary_id: String,
    team_num: u8,
    score: u16,
    shortcut: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiTeamData {
    score: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiPlayerTargetData {
    shortcut: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiGameData {
    teams: [StatsApiTeamData; 2],
    arena: String,
    #[serde(rename = "bOvertime")]
    overtime: bool,
    #[serde(rename = "bReplay")]
    replay: bool,
    target: Option<StatsApiPlayerTargetData>,
}

impl StatsApiGameData {
    fn scores(&self) -> TeamScores {
        TeamScores {
            blue: self.teams[0].score,
            orange: self.teams[1].score,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UpdateStateEventData {
    players: Vec<StatsApiPlayerData>,
    game: StatsApiGameData,
}

#[derive(Debug, Default, Clone)]
pub struct TeamScores {
    pub blue: u8,
    pub orange: u8,
}

impl TeamScores {
    pub fn guess_winner(&self) -> Option<Team> {
        Some(match self.blue.cmp(&self.orange) {
            Ordering::Equal => return None,
            Ordering::Greater => Team::Blue,
            Ordering::Less => Team::Orange,
        })
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Team {
    Blue,
    Orange,
}

impl From<u8> for Team {
    fn from(value: u8) -> Self {
        match value {
            0 => Team::Blue,
            1 => Team::Orange,
            _ => unreachable!("invalid team {}", value),
        }
    }
}

impl fmt::Display for Team {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Team::Blue => "Blue",
                Team::Orange => "Orange",
            }
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MatchEndedEventData {
    winner_team_num: u8,
}

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub name: String,
    pub platform: Platform,
    pub platform_id: String,
    pub team: Team,
    pub score: u16,
}

fn parse_stats_api_player(player: StatsApiPlayerData) -> Option<PlayerData> {
    let parts: Vec<&str> = player.primary_id.split('|').collect();

    if let Ok(platform) = Platform::from_str(parts[0]) {
        Some(PlayerData {
            name: player.name,
            platform,
            platform_id: player.primary_id,
            team: player.team_num.into(),
            score: player.score,
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

#[derive(Debug, Clone)]
pub struct MatchUpdate {
    pub score: TeamScores,
    pub players: Vec<PlayerData>,
    pub arena: &'static str,
    pub state: MatchState,
}

pub enum RLEvent {
    Update(MatchUpdate),
    MatchStart,
    MatchOver(Team), // winner
    MatchLeft,

    ReplayStart,
    ReplayDone,

    Connected,
    Disconnected,

    OurPlayerId(String),
}

enum ConnectionState {
    Connected(TcpStream),
    Disconnected,
}

pub struct StatsApi {
    connection: ConnectionState,
    read_buffer: [u8; 4096],
    local_player_id_event_emitted_yet: bool,
    match_created_event_happened: bool,
}

impl StatsApi {
    pub fn new() -> Self {
        StatsApi {
            connection: ConnectionState::Disconnected,
            read_buffer: [0; 4096],
            local_player_id_event_emitted_yet: false,
            match_created_event_happened: false,
        }
    }

    pub fn update(&mut self) -> Option<RLEvent> {
        match &mut self.connection {
            ConnectionState::Connected(stream) => {
                let n_bytes = match stream.read(&mut self.read_buffer) {
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        return None;
                    }
                    Ok(0) | Err(_) => {
                        self.connection = ConnectionState::Disconnected;
                        self.match_created_event_happened = false;
                        return Some(RLEvent::Disconnected);
                    }
                    Ok(b) => b,
                };

                let Ok(text) = std::str::from_utf8(&self.read_buffer[..n_bytes]) else {
                    eprintln!("Failed to decode...");
                    return None;
                };

                let Ok(event) = serde_json::from_str::<StatsApiEvent>(text) else {
                    // ignore (probably framing issue)
                    return None;
                };

                self.on_api_event(event)
            }
            ConnectionState::Disconnected => {
                if let Ok(new_stream) = TcpStream::connect_timeout(
                    &"127.0.0.1:49123".parse::<SocketAddr>().unwrap(),
                    Duration::from_millis(3),
                ) {
                    new_stream
                        .set_nonblocking(true)
                        .expect("set_nonblocking call failed");
                    self.connection = ConnectionState::Connected(new_stream);
                    Some(RLEvent::Connected)
                } else {
                    None
                }
            }
        }
    }

    fn on_api_event(&mut self, event: StatsApiEvent) -> Option<RLEvent> {
        match event.event.as_str() {
            "UpdateState" => {
                let data: UpdateStateEventData = serde_json::from_str(&event.data).unwrap();

                if !self.local_player_id_event_emitted_yet
                    && let Some(game_target) = data.game.target.as_ref()
                {
                    let target_shortcut = game_target.shortcut;
                    let our_player = data.players.iter().find(|p| p.shortcut == target_shortcut);
                    if let Some(player) = our_player {
                        self.local_player_id_event_emitted_yet = true;
                        return Some(RLEvent::OurPlayerId(player.primary_id.clone()));
                    }
                }

                Some(RLEvent::Update(MatchUpdate {
                    state: if data.game.replay {
                        MatchState::Replay
                    } else if data.game.overtime {
                        MatchState::Overtime
                    } else {
                        MatchState::Game
                    },
                    score: data.game.scores(),
                    arena: super::asset_to_arena(&data.game.arena).unwrap_or("Unknown"),
                    players: data
                        .players
                        .into_iter()
                        .filter_map(parse_stats_api_player)
                        .collect(),
                }))
            }
            "MatchCreated" => {
                self.match_created_event_happened = true;
                None
            }
            "CountdownBegin" if self.match_created_event_happened => {
                self.match_created_event_happened = false;
                Some(RLEvent::MatchStart)
            }
            "MatchEnded" => {
                let data: MatchEndedEventData = serde_json::from_str(&event.data).unwrap();
                Some(RLEvent::MatchOver(Team::from(data.winner_team_num)))
            }
            "MatchDestroyed" => Some(RLEvent::MatchLeft),
            "GoalReplayStart" => Some(RLEvent::ReplayStart),
            "GoalReplayEnd" => Some(RLEvent::ReplayDone),
            _ => None,
        }
    }
}
