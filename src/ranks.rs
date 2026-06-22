use std::{
    collections::HashMap,
    sync::{Arc, RwLock, mpsc},
    thread,
};

use eframe::egui;
use serde::Deserialize;

use crate::rl_stats_api::PlayerData;

const API_URL: &str = "https://mmr.kmdw.dev/get-skills";

#[derive(Clone)]
#[repr(u8)]
enum Playlist {
    Ones = 10,
    Twos = 11,
    Threes = 13,
}

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsPlaylistData {
    id: u8,
    mmr: i16,
    tier: u8,
    // division: u8, - this exists maybe use in the future
}

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsResponse {
    playlists: Vec<GetPlayerSkillsPlaylistData>,
}

impl GetPlayerSkillsResponse {
    pub fn get_playlist(&self, playlist: Playlist) -> Option<&GetPlayerSkillsPlaylistData> {
        let playlist_id = playlist as u8;
        self.playlists.iter().find(|sk| sk.id == playlist_id)
    }
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
#[allow(dead_code)] // since its constructed with mem::transmute
pub enum Rank {
    Unranked,
    Bronze1,
    Bronze2,
    Bronze3,
    Silver1,
    Silver2,
    Silver3,
    Gold1,
    Gold2,
    Gold3,
    Plat1,
    Plat2,
    Plat3,
    Diamond1,
    Diamond2,
    Diamond3,
    Champ1,
    Champ2,
    Champ3,
    GC1,
    GC2,
    GC3,
    Ssl,
}

impl Rank {
    pub fn as_str(&self) -> &'static str {
        match self {
            Rank::Unranked => "Unranked",
            Rank::Bronze1 => "Bronze I",
            Rank::Bronze2 => "Bronze II",
            Rank::Bronze3 => "Bronze III",
            Rank::Silver1 => "Silver I",
            Rank::Silver2 => "Silver II",
            Rank::Silver3 => "Silver III",
            Rank::Gold1 => "Gold I",
            Rank::Gold2 => "Gold II",
            Rank::Gold3 => "Gold III",
            Rank::Plat1 => "Platinum I",
            Rank::Plat2 => "Platinum II",
            Rank::Plat3 => "Platinum III",
            Rank::Diamond1 => "Diamond I",
            Rank::Diamond2 => "Diamond II",
            Rank::Diamond3 => "Diamond III",
            Rank::Champ1 => "Champion I",
            Rank::Champ2 => "Champion II",
            Rank::Champ3 => "Champion III",
            Rank::GC1 => "Grand Champion I",
            Rank::GC2 => "Grand Champion II",
            Rank::GC3 => "Grand Champion III",
            Rank::Ssl => "Supersonic Legend",
        }
    }

    pub fn from_tier(tier: u8) -> Rank {
        match tier {
            // rust should have a non unsafe way to do it automatically tbh
            0..=22 => unsafe { std::mem::transmute::<u8, Rank>(tier) },
            _ => unreachable!("invalid tier: {}", tier),
        }
    }

    // uses f2p season 23 1v1
    pub fn estimate_from_mmr(mmr: i16) -> Rank {
        match mmr {
            ..=156 => Rank::Bronze1,
            ..=213 => Rank::Bronze2,
            ..=274 => Rank::Bronze3,
            ..=334 => Rank::Silver1,
            ..=394 => Rank::Silver2,
            ..=454 => Rank::Silver3,
            ..=514 => Rank::Gold1,
            ..=574 => Rank::Gold2,
            ..=634 => Rank::Gold3,
            ..=694 => Rank::Plat1,
            ..=753 => Rank::Plat2,
            ..=808 => Rank::Plat3,
            ..=874 => Rank::Diamond1,
            ..=930 => Rank::Diamond2,
            ..=994 => Rank::Diamond3,
            ..=1052 => Rank::Champ1,
            ..=1114 => Rank::Champ2,
            ..=1170 => Rank::Champ3,
            ..=1232 => Rank::GC1,
            ..=1295 => Rank::GC2,
            ..=1351 => Rank::GC3,
            _ => Rank::Ssl,
        }
    }
}

#[derive(Debug)]
pub struct PlayerSkillInformation {
    pub rank: Rank,
    pub mmr: i16,
    pub rank_is_estimate: bool,
}

impl PlayerSkillInformation {
    fn for_bot() -> PlayerSkillInformation {
        PlayerSkillInformation {
            rank: Rank::Unranked,
            mmr: 0,
            rank_is_estimate: false,
        }
    }

    fn from_playlist(playlist: &GetPlayerSkillsPlaylistData) -> PlayerSkillInformation {
        let actual_rank = Rank::from_tier(playlist.tier);
        let use_estimate = actual_rank == Rank::Unranked;

        PlayerSkillInformation {
            rank: if use_estimate {
                Rank::estimate_from_mmr(playlist.mmr)
            } else {
                actual_rank
            },
            mmr: playlist.mmr,
            rank_is_estimate: use_estimate,
        }
    }
}

#[derive(Debug)]
pub struct EventRanks {
    pub ranked_1s: Option<PlayerSkillInformation>,
    pub ranked_2s: Option<PlayerSkillInformation>,
    pub ranked_3s: Option<PlayerSkillInformation>,
}

impl EventRanks {
    fn from_skills(skill: GetPlayerSkillsResponse) -> EventRanks {
        EventRanks {
            ranked_1s: skill
                .get_playlist(Playlist::Ones)
                .map(PlayerSkillInformation::from_playlist),
            ranked_2s: skill
                .get_playlist(Playlist::Twos)
                .map(PlayerSkillInformation::from_playlist),
            ranked_3s: skill
                .get_playlist(Playlist::Threes)
                .map(PlayerSkillInformation::from_playlist),
        }
    }
}

fn get_with_retries<const RETRIES: u8>(
    url: &String,
) -> Result<ureq::http::Response<ureq::Body>, ()> {
    for _ in 0..RETRIES {
        match ureq::get(url).call() {
            Ok(resp) => return Ok(resp),
            Err(_) => continue,
        }
    }

    Err(())
}

pub struct RankAPI {
    // key is stringified PlayerData
    // option for whether its loaded yet
    ranks: Arc<RwLock<HashMap<String, Option<Arc<EventRanks>>>>>,
    context: egui::Context,
    error_sender: mpsc::Sender<String>,
}

impl RankAPI {
    pub fn new(context: egui::Context, error_tx: mpsc::Sender<String>) -> RankAPI {
        RankAPI {
            ranks: Arc::new(RwLock::new(HashMap::new())),
            context,
            error_sender: error_tx,
        }
    }

    pub fn get(&self, player: &PlayerData) -> Option<Arc<EventRanks>> {
        let player_key = player.to_string();

        let current = Arc::clone(&self.ranks);
        if let Some(existing) = current.read().unwrap().get(&player_key) {
            return existing.clone();
        }

        let context = self.context.clone();
        let error_tx = self.error_sender.clone();

        let url = format!(
            "{}?playerId={}",
            API_URL,
            urlencoding::encode(&player.platform_id)
        );
        let player_platform = player.platform;

        thread::spawn(move || {
            // drop the lock before making the http request
            {
                let mut current = current.write().unwrap();
                if player_platform == crate::rl_stats_api::Platform::Bot {
                    current.insert(
                        player_key,
                        Some(Arc::new(EventRanks {
                            ranked_1s: Some(PlayerSkillInformation::for_bot()),
                            ranked_2s: Some(PlayerSkillInformation::for_bot()),
                            ranked_3s: Some(PlayerSkillInformation::for_bot()),
                        })),
                    );
                    context.request_repaint();
                    return;
                }

                current.insert(player_key.clone(), None);
            }

            let Ok(mut response) = get_with_retries::<3>(&url) else {
                error_tx
                    .send("Could not communicate with rank server".to_string())
                    .unwrap();
                return;
            };

            let response = response
                .body_mut()
                .read_json::<GetPlayerSkillsResponse>()
                .unwrap();

            let ranks = EventRanks::from_skills(response);

            let mut current = current.write().unwrap();
            current.insert(player_key.clone(), Some(Arc::new(ranks)));
            context.request_repaint();
        });

        None
    }
}
