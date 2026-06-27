use std::{
    collections::HashMap,
    sync::{Arc, RwLock, mpsc},
    thread,
};

use eframe::egui;
use num_enum::TryFromPrimitive;
use serde::Deserialize;

use crate::core::{Division, Playlist, Rank};

const API_URL: &str = "https://mmr.kmdw.dev/get-skills";

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsPlaylistData {
    id: u8,
    mmr: i16,
    tier: u8,
    division: u8,
}

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsResponse {
    playlists: Vec<GetPlayerSkillsPlaylistData>,
}

impl GetPlayerSkillsResponse {
    pub fn get_playlist(&self, playlist: Playlist) -> Option<&GetPlayerSkillsPlaylistData> {
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

fn get_with_retries<const RETRIES: u8>(
    url: &String,
) -> Result<ureq::http::Response<ureq::Body>, ()> {
    for _ in 0..RETRIES {
        if let Ok(resp) = ureq::get(url).call() {
            return Ok(resp);
        }
    }

    Err(())
}

pub struct RankAPI {
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

    // String reference to only clone it if we actually need the ownership for
    // a new thread
    pub fn get(&self, platform_id: &String) -> Option<Arc<EventRanks>> {
        let current = Arc::clone(&self.ranks);
        if let Some(existing) = current.read().unwrap().get(platform_id) {
            return existing.clone();
        }

        let platform_id = platform_id.clone();
        let context = self.context.clone();
        let error_tx = self.error_sender.clone();

        let url = format!("{}?playerId={}", API_URL, urlencoding::encode(&platform_id));

        thread::spawn(move || {
            // drop the lock before making the http request
            {
                let mut current = current.write().unwrap();
                current.insert(platform_id.clone(), None);
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

            let ranks = EventRanks::from_skills(&response);

            let mut current = current.write().unwrap();
            current.insert(platform_id, Some(Arc::new(ranks)));
            context.request_repaint();
        });

        None
    }
}
