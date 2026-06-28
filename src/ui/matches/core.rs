use std::time::SystemTime;

use crate::rl::{PlayerData, Team, TeamScores};

#[derive(Debug, Clone)]
pub struct MatchPlayer {
    pub left: bool,
    pub data: PlayerData,
}

impl From<PlayerData> for MatchPlayer {
    fn from(value: PlayerData) -> Self {
        MatchPlayer {
            left: false,
            data: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchOverInfo {
    pub timestamp: SystemTime,
    pub winner: Option<Team>,
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub players: Vec<MatchPlayer>,
    pub score: TeamScores,
    pub our_team: Team,
    pub finish: Option<MatchOverInfo>,
    pub started_at: SystemTime,
}

impl Default for MatchInfo {
    fn default() -> Self {
        MatchInfo {
            players: Vec::new(),
            score: TeamScores { blue: 0, orange: 0 },
            our_team: Team::Blue,
            finish: None,
            started_at: SystemTime::now(),
        }
    }
}
