use std::sync::Arc;

use eframe::egui;
use num_enum::TryFromPrimitive as _;
use serde::Deserialize;

use crate::{
    common::CachedHttpApi,
    rocket_league::{Division, Playlist, Rank},
};

const API_URL: &str = "https://mmr.kmdw.dev/get-skills";

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsPlaylistData {
    id: u8,
    mmr: i16,
    tier: u8,
    division: u8,
}

#[derive(Deserialize, Debug)]
pub struct GetPlayerSkillsResponse {
    playlists: Vec<GetPlayerSkillsPlaylistData>,
}

impl GetPlayerSkillsResponse {
    fn get_playlist(&self, playlist: Playlist) -> Option<&GetPlayerSkillsPlaylistData> {
        let playlist_id: u8 = playlist.into();
        self.playlists.iter().find(|sk| sk.id == playlist_id)
    }
}

#[derive(Debug)]
pub struct PlayerSkillInformation {
    pub rank: Rank,
    pub div: Division,
    pub mmr: i16,
    pub rank_is_estimate: bool,
}

impl PlayerSkillInformation {
    fn from_playlist(playlist: &GetPlayerSkillsPlaylistData) -> PlayerSkillInformation {
        let actual_rank = Rank::try_from_primitive(playlist.tier).expect("Failed to convert rank");
        let use_estimate = actual_rank == Rank::Unranked;

        PlayerSkillInformation {
            rank: if use_estimate {
                Rank::estimate_from_mmr(playlist.mmr)
            } else {
                actual_rank
            },
            div: Division::from(playlist.division),
            mmr: playlist.mmr,
            rank_is_estimate: use_estimate,
        }
    }
}

#[derive(Debug)]
pub struct EventRanks {
    pub duels: Option<PlayerSkillInformation>,
    pub doubles: Option<PlayerSkillInformation>,
    pub standard: Option<PlayerSkillInformation>,
}

impl EventRanks {
    fn from_skills(skill: &GetPlayerSkillsResponse) -> EventRanks {
        EventRanks {
            duels: skill
                .get_playlist(Playlist::Ones)
                .map(PlayerSkillInformation::from_playlist),
            doubles: skill
                .get_playlist(Playlist::Twos)
                .map(PlayerSkillInformation::from_playlist),
            standard: skill
                .get_playlist(Playlist::Threes)
                .map(PlayerSkillInformation::from_playlist),
        }
    }
}

pub type RankAPI = CachedHttpApi<String, EventRanks, GetPlayerSkillsResponse>;

pub fn new_rank_api(context: egui::Context) -> RankAPI {
    CachedHttpApi::new(
        context,
        Box::new(|player_id| format!("{}?playerId={}", API_URL, urlencoding::encode(player_id))),
        Arc::new(|response| Some(EventRanks::from_skills(&response))),
    )
}
