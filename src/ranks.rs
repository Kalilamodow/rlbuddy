use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
};

use eframe::egui;
use serde::Deserialize;

use crate::rl_stats_api::{Platform, PlayerData};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)\
AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[repr(u8)]
enum PlaylistID {
    Ones = 10,
    Twos = 11,
    // idk why its not 12
    Threes = 13,
}

#[derive(Deserialize)]
struct TrackerGGProfileResponseDataSegment {
    #[serde(rename = "type")]
    segment_type: String,

    // for other segment types these two can be different
    // im not deserializing them because its easier to just use bracket indexing
    attributes: serde_json::Value,
    stats: serde_json::Value,
}

#[derive(Deserialize)]
struct TrackerGGProfileResponseData {
    segments: Vec<TrackerGGProfileResponseDataSegment>,
}

#[derive(Deserialize)]
struct TrackerGGProfileResponse {
    data: TrackerGGProfileResponseData,
}

#[derive(Debug, Clone)]
pub struct EventRanks {
    pub ranked_1s: Option<String>,
    pub ranked_2s: Option<String>,
    pub ranked_3s: Option<String>,
}

pub struct PlayerRankInformation {
    // key is stringified PlayerData
    // option for whether its loaded yet
    ranks: Arc<RwLock<HashMap<String, Option<Arc<EventRanks>>>>>,
    context: egui::Context,
}

fn playlist_segment_tier_by_playlist(
    segments: &mut core::slice::Iter<TrackerGGProfileResponseDataSegment>,
    playlist_id: u8,
) -> Option<String> {
    segments
        .find(|seg| seg.segment_type == "playlist" && seg.attributes["playlistId"] == playlist_id)
        .map(|playlist| {
            playlist.stats["tier"]["metadata"]["name"]
                .as_str()
                .unwrap()
                .to_string()
        })
}

impl PlayerRankInformation {
    pub fn new(context: egui::Context) -> PlayerRankInformation {
        PlayerRankInformation {
            ranks: Arc::new(RwLock::new(HashMap::new())),
            context,
        }
    }

    pub fn get(&self, player: &PlayerData) -> Option<Arc<EventRanks>> {
        let player_key = player.to_string();

        let current = Arc::clone(&self.ranks);
        if let Some(existing) = current.read().unwrap().get(&player_key) {
            return existing.clone();
        }

        let url = format!(
            "https://api.tracker.gg/api/v2/rocket-league/standard/profile/{}/{}",
            match player.platform {
                Platform::Epic => "epic",
                Platform::Steam => "steam",
                Platform::Xbox => "xbl",
                Platform::PlayStation => "psn",
            },
            match player.platform {
                Platform::Epic | Platform::Xbox | Platform::PlayStation =>
                    urlencoding::encode(&player.name).into_owned(),
                Platform::Steam => player.platform_id.clone(),
            }
        );

        let context = self.context.clone();
        thread::spawn(move || {
            let mut current = current.write().unwrap();
            current.insert(player_key.clone(), None);

            let response = ureq::get(url)
                .header("User-Agent", USER_AGENT)
                .call()
                .unwrap()
                .body_mut()
                .read_json::<TrackerGGProfileResponse>()
                .unwrap();

            let mut segments = response.data.segments.iter();

            let ranks = EventRanks {
                ranked_1s: playlist_segment_tier_by_playlist(&mut segments, PlaylistID::Ones as u8),
                ranked_2s: playlist_segment_tier_by_playlist(&mut segments, PlaylistID::Twos as u8),
                ranked_3s: playlist_segment_tier_by_playlist(
                    &mut segments,
                    PlaylistID::Threes as u8,
                ),
            };

            current.insert(player_key.clone(), Some(Arc::new(ranks)));
            context.request_repaint();
        });

        None
    }
}
