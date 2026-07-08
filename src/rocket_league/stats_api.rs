use serde::Deserialize;
use std::{
    cmp::Ordering,
    fmt,
    io::Read,
    net::{SocketAddr, TcpStream},
    str::FromStr,
};

use crate::rocket_league::{MatchState, RLEvent::OurPlayerId};

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

#[derive(Debug)]
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

// cant use connect_timeout bc it just errors instead of waiting when the
// socket isnt open in the first place
fn connect_forever() -> TcpStream {
    loop {
        if let Ok(tcp) = TcpStream::connect("127.0.0.1:49123".parse::<SocketAddr>().unwrap()) {
            return tcp;
        }
    }
}

pub fn connect_to_stats_api<F: Fn(RLEvent)>(on_event: F) {
    let mut read_buffer = vec![0u8; 4096];
    let mut local_player_id_event_emitted_yet = false;

    loop {
        let mut tcp = connect_forever();

        // MatchInitialized doesnt fire in private matches for some reason
        // so listen for match created then the first countdown is the "game start"
        let mut match_created_event_happened = false;
        on_event(RLEvent::Connected);

        loop {
            let n_bytes = match tcp.read(&mut read_buffer) {
                Ok(0) => continue,
                Ok(b) => b,
                Err(_) => {
                    on_event(RLEvent::Disconnected);
                    break;
                }
            };

            let Ok(text) = std::str::from_utf8(&read_buffer[..n_bytes]) else {
                eprintln!("Failed to decode...");
                continue;
            };

            let Ok(event) = serde_json::from_str::<StatsApiEvent>(text) else {
                // ignore (probably framing issue)
                continue;
            };

            match event.event.as_str() {
                "UpdateState" => {
                    let data: UpdateStateEventData = serde_json::from_str(&event.data).unwrap();

                    if !local_player_id_event_emitted_yet
                        && let Some(game_target) = data.game.target.as_ref()
                    {
                        let target_shortcut = game_target.shortcut;
                        let our_player =
                            data.players.iter().find(|p| p.shortcut == target_shortcut);
                        if let Some(player) = our_player {
                            on_event(OurPlayerId(player.primary_id.clone()));
                            local_player_id_event_emitted_yet = true;
                        }
                    }

                    on_event(RLEvent::Update(MatchUpdate {
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
                    }));
                }
                "MatchCreated" => {
                    match_created_event_happened = true;
                }
                "CountdownBegin" if match_created_event_happened => {
                    match_created_event_happened = false;
                    on_event(RLEvent::MatchStart);
                }
                "MatchEnded" => {
                    let data: MatchEndedEventData = serde_json::from_str(&event.data).unwrap();
                    on_event(RLEvent::MatchOver(Team::from(data.winner_team_num)));
                }
                "MatchDestroyed" => {
                    on_event(RLEvent::MatchLeft);
                }
                "GoalReplayStart" => {
                    on_event(RLEvent::ReplayStart);
                }
                "GoalReplayEnd" => {
                    on_event(RLEvent::ReplayDone);
                }
                _ => {}
            }
        }
    }
}
