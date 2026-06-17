use std::{
    collections::HashMap,
    sync::{Arc, RwLock, mpsc},
    thread,
};

use eframe::egui;
use serde::Deserialize;

use crate::rl_stats_api::PlayerData;

const API_URL: &str = "https://rocket-league-mmrs.kmdw.dev";

#[derive(Clone)]
#[repr(u8)]
enum Playlist {
    Ones = 10,
    Twos = 11,
    // idk why its not 12
    Threes = 13,
}

// Skill rating = Mu * 20 + 100
// https://www.reddit.com/r/RocketLeague/comments/juuzkn/comment/gchywcr/?context=3
// note: throughout the code we refer to skill rating by its informal name, mmr
fn mu_to_skill_rating(mu: f64) -> i16 {
    let real = mu * 20.0 + 100.0;
    real.ceil() as i16
}

// for the sake of readability, all unused fields are ignored
#[derive(Deserialize, Debug)]
struct GetPlayerSkillsResponseSkill {
    #[serde(rename = "Playlist")]
    playlist: u8,
    #[serde(rename = "Tier")]
    tier: u8,
    // see comment in fn mu_to_skill_rating
    #[serde(rename = "Mu")]
    mu: f64,
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
pub struct PlayerSkillInformation {
    pub rank: Rank,
    pub mmr: i16,
    pub rank_is_estimate: bool,
}

impl PlayerSkillInformation {
    pub fn for_bot() -> PlayerSkillInformation {
        PlayerSkillInformation {
            rank: Rank::Unranked,
            mmr: 0,
            rank_is_estimate: false,
        }
    }
}

#[derive(Debug)]
pub struct EventRanks {
    pub ranked_1s: Option<PlayerSkillInformation>,
    pub ranked_2s: Option<PlayerSkillInformation>,
    pub ranked_3s: Option<PlayerSkillInformation>,
}

fn tier_to_rank(tier: u8) -> Rank {
    match tier {
        // rust should have a non unsafe way to do it automatically tbh
        0..=22 => unsafe { std::mem::transmute::<u8, Rank>(tier) },
        _ => unreachable!("invalid tier: {}", tier),
    }
}

/// uses ftp season 23 1v1
fn mmr_rank_estimate(mmr: &i16) -> Rank {
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
        _ => Rank::SSL,
    }
}

fn skill_by_playlist(
    skills: &Vec<GetPlayerSkillsResponseSkill>,
    playlist: Playlist,
) -> Option<PlayerSkillInformation> {
    let playlist_id = playlist as u8;
    skills
        .iter()
        .find(|sk| sk.playlist == playlist_id)
        .map(|sk| PlayerSkillInformation {
            rank: tier_to_rank(sk.tier),
            mmr: mu_to_skill_rating(sk.mu),
            rank_is_estimate: false,
        })
        .map(|sk| {
            if sk.rank == Rank::Unranked {
                PlayerSkillInformation {
                    rank: mmr_rank_estimate(&sk.mmr),
                    mmr: sk.mmr,
                    rank_is_estimate: true,
                }
            } else {
                sk
            }
        })
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
            "{}/skills/getPlayerSkill/{}",
            API_URL,
            urlencoding::encode(&player.platform_id)
        );
        let player_platform = player.platform;

        thread::spawn(move || {
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
                ranked_1s: skill_by_playlist(&response.skill.skills, Playlist::Ones),
                ranked_2s: skill_by_playlist(&response.skill.skills, Playlist::Twos),
                ranked_3s: skill_by_playlist(&response.skill.skills, Playlist::Threes),
            };

            current.insert(player_key.clone(), Some(Arc::new(ranks)));
            context.request_repaint();
        });

        None
    }
}
