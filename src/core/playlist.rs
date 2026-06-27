use std::fmt;

use num_enum::IntoPrimitive;

#[derive(Clone, IntoPrimitive)]
#[repr(u8)]
pub enum Playlist {
    Ones = 10,
    Twos = 11,
    Threes = 13,
}

impl Playlist {
    pub fn from_player_count(player_count: usize) -> Option<Playlist> {
        match player_count {
            2 => Some(Playlist::Ones),
            4 => Some(Playlist::Twos),
            6 => Some(Playlist::Threes),
            _ => None,
        }
    }
}

impl fmt::Display for Playlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Playlist::Ones => "1s",
                Playlist::Twos => "2s",
                Playlist::Threes => "3s",
            }
        )
    }
}
