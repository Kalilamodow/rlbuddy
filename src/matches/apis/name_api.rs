// Basically for uncensoring names

use crate::common::CachedHttpApi;
use eframe::egui;
use serde::Deserialize;
use std::sync::Arc;

const API_URL: &str = "https://mmr.kmdw.dev/get-profile";

#[derive(Debug, Deserialize)]
pub struct GetProfileResponse {
    name: String,
    // id: String,
    // state: String,
}

pub type NameAPI = CachedHttpApi<String, String, GetProfileResponse>;

pub fn new_name_api(context: egui::Context) -> NameAPI {
    CachedHttpApi::new(
        context,
        Box::new(|player_id| format!("{}?playerId={}", API_URL, urlencoding::encode(player_id))),
        Arc::new(|resp| Some(resp.name)),
    )
}
