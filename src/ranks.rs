use std::{
    collections::HashMap,
    sync::{Arc, RwLock, mpsc},
    thread,
};

use eframe::egui;
use serde::Deserialize;

use crate::rl_stats_api::PlayerData;

const API_URL: &str = "https://rocket-league-mmrs.kmdw.dev";

#[repr(u8)]
enum PlaylistID {
    Ones = 10,
    Twos = 11,
    // idk why its not 12
    Threes = 13,
}

// for the sake of readability, all unused fields are ignored
#[derive(Deserialize, Debug)]
struct GetPlayerSkillsResponseSkill {
    #[serde(rename = "Playlist")]
    playlist: u8,
    #[serde(rename = "Tier")]
    tier: u8,
}

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsResponseData {
    #[serde(rename = "Skills")]
    skills: Vec<GetPlayerSkillsResponseSkill>,
}

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsResponse {
    skill: GetPlayerSkillsResponseData,
}

#[derive(Debug, Clone)]
pub struct EventRanks {
    pub ranked_1s: Option<&'static str>,
    pub ranked_2s: Option<&'static str>,
    pub ranked_3s: Option<&'static str>,
}

fn tier_to_rank(tier: u8) -> &'static str {
    match tier {
        0 => "Unranked",
        1 => "Bronze 1",
        2 => "Bronze 2",
        3 => "Bronze 3",
        4 => "Silver 1",
        5 => "Silver 2",
        6 => "Silver 3",
        7 => "Gold 1",
        8 => "Gold 2",
        9 => "Gold 3",
        10 => "Platinum 1",
        11 => "Platinum 2",
        12 => "Platinum 3",
        13 => "Diamond 1",
        14 => "Diamond 2",
        15 => "Diamond 3",
        16 => "Champ 1",
        17 => "Champ 2",
        18 => "Champ 3",
        19 => "GC 1",
        20 => "GC 2",
        21 => "GC 3",
        22 => "SSL",
        _ => unreachable!("invalid tier: {}", tier),
    }
}

fn rank_by_playlist(
    skills: &Vec<GetPlayerSkillsResponseSkill>,
    playlist: u8,
) -> Option<&'static str> {
    skills
        .iter()
        .find(|sk| sk.playlist == playlist)
        .map(|sk| tier_to_rank(sk.tier))
}

pub struct PlayerRankInformation {
    // key is stringified PlayerData
    // option for whether its loaded yet
    ranks: Arc<RwLock<HashMap<String, Option<Arc<EventRanks>>>>>,
    context: egui::Context,
    error_sender: mpsc::Sender<String>,
}

impl PlayerRankInformation {
    pub fn new(context: egui::Context, error_tx: mpsc::Sender<String>) -> PlayerRankInformation {
        PlayerRankInformation {
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

        let url = format!(
            "{}/skills/getPlayerSkill/{}",
            API_URL,
            urlencoding::encode(&player.platform_id)
        );

        let context = self.context.clone();
        let error_tx = self.error_sender.clone();
        thread::spawn(move || {
            let mut current = current.write().unwrap();
            current.insert(player_key.clone(), None);

            let Ok(mut response) = ureq::get(url).call() else {
                error_tx
                    .send("Could not communicate with rank server".to_string())
                    .unwrap();
                return;
            };

            let response = response
                .body_mut()
                .read_json::<GetPlayerSkillsResponse>()
                .unwrap();

            println!("response: {:#?}", response.skill.skills);

            let ranks = EventRanks {
                ranked_1s: rank_by_playlist(&response.skill.skills, PlaylistID::Ones as u8),
                ranked_2s: rank_by_playlist(&response.skill.skills, PlaylistID::Twos as u8),
                ranked_3s: rank_by_playlist(&response.skill.skills, PlaylistID::Threes as u8),
            };

            current.insert(player_key.clone(), Some(Arc::new(ranks)));
            context.request_repaint();
        });

        None
    }
}
