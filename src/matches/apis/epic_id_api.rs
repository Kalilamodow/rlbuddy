// platform id -> epic name

use crate::common::CachedHttpApi;
use eframe::egui;
use serde::Deserialize;
use std::sync::Arc;

const API_URL: &str = "https://mmr.kmdw.dev/player-id-to-epic-name";

#[derive(Debug, Deserialize)]
pub struct EpicIdResponse {
    name: Option<String>,
}

pub type EpicIdAPI = CachedHttpApi<String, String, EpicIdResponse>;

pub fn new_epic_id_api(context: egui::Context) -> EpicIdAPI {
    CachedHttpApi::new(
        context,
        Box::new(|player_id| format!("{}?playerId={}", API_URL, urlencoding::encode(player_id))),
        Arc::new(|resp| resp.name),
    )
}
