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

#[derive(Debug)]
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
    SSL,
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
            Rank::SSL => "Supersonic Legend",
        }
    }
}

#[derive(Debug)]
pub struct EventRanks {
    pub ranked_1s: Option<Rank>,
    pub ranked_2s: Option<Rank>,
    pub ranked_3s: Option<Rank>,
}

fn tier_to_rank(tier: u8) -> Rank {
    match tier {
        // rust should have a non unsafe way to do it automatically tbh
        0..=22 => unsafe { std::mem::transmute::<u8, Rank>(tier) },
        _ => unreachable!("invalid tier: {}", tier),
    }
}

fn rank_by_playlist(skills: &Vec<GetPlayerSkillsResponseSkill>, playlist: u8) -> Option<Rank> {
    skills
        .iter()
        .find(|sk| sk.playlist == playlist)
        .map(|sk| tier_to_rank(sk.tier))
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
