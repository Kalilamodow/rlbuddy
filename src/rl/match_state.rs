#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchState {
    Game,
    Replay,
    Overtime,
}

impl MatchState {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchState::Game => "In game",
            MatchState::Replay => "Watching replay",
            MatchState::Overtime => "In overtime",
        }
    }
}
