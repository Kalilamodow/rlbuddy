use std::sync::Arc;

use eframe::egui;
use serde::Deserialize;

use crate::{common::CachedHttpApi, rocket_league::Platform, stats_api::PlayerData};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankIconMetadata {
    pub rank_name: String,
}

#[derive(Debug, Deserialize)]
pub struct DefaultMetadata {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentStat {
    pub display_name: String,
    pub display_value: Option<String>,
}

const NONE_STRING: &str = "-";

impl SegmentStat {
    pub fn value(&self) -> &str {
        self.display_value
            .as_ref()
            .map_or(NONE_STRING, |v| v.as_str())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSegmentStatsStatWithMetadata {
    pub display_name: String,
    pub metadata: RankIconMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSegmentStats {
    pub wins: SegmentStat,
    pub goals: SegmentStat,
    pub season_reward_level: OverviewSegmentStatsStatWithMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSegment {
    pub stats: OverviewSegmentStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSegmentStatsStatWithMetadata {
    pub metadata: DefaultMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSegmentStats {
    pub tier: PlaylistSegmentStatsStatWithMetadata,
    pub win_streak: SegmentStat,
    pub rating: SegmentStat,
    pub peak_rating: SegmentStat,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSegment {
    pub metadata: DefaultMetadata,
    pub stats: PlaylistSegmentStats,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Segment {
    Overview(OverviewSegment),
    Playlist(PlaylistSegment),
    PlaylistAverage,
    #[serde(rename = "peak-rating")]
    PeakRating,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub platform_user_handle: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileData {
    pub platform_info: PlatformInfo,
    pub segments: Vec<Segment>,
}

#[derive(Debug)]
pub enum TRNError {
    NotFound,
    Other(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseData {
    pub code: String,
}

impl From<ErrorResponseData> for TRNError {
    fn from(value: ErrorResponseData) -> Self {
        match value.code.as_str() {
            "CollectorResultStatus::NotFound" => TRNError::NotFound,
            _ => TRNError::Other(value.code),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    Data(ProfileData),
    Errors(Vec<ErrorResponseData>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerKey {
    pub platform: Platform,
    pub platform_id: String,
}

impl PlayerKey {
    fn to_api_url(&self) -> String {
        let prefix = match self.platform {
            Platform::Xbox => "xbl",
            Platform::Epic => "epic",
            Platform::PlayStation => "psn",
            Platform::Steam => "steam",
            Platform::Switch => "switch",
            Platform::Bot => unreachable!("trying to get player key of bot"),
        };

        format!(
            "https://api.tracker.gg/api/v2/rocket-league/standard/profile/{prefix}/{}",
            urlencoding::encode(&self.platform_id)
        )
    }
}

impl From<PlayerData> for PlayerKey {
    fn from(value: PlayerData) -> Self {
        Self {
            platform: value.platform,
            platform_id: match value.platform {
                Platform::Steam => value.platform_id,
                _ => value.name,
            },
        }
    }
}

pub type TrackerAPI = CachedHttpApi<PlayerKey, Result<ProfileData, TRNError>, ProfileResponse>;

pub fn new_tracker_api(context: egui::Context) -> TrackerAPI {
    CachedHttpApi::new(
        context,
        Box::new(|key| key.to_api_url()),
        Arc::new(|r| {
            Some(match r {
                ProfileResponse::Data(d) => Ok(d),
                ProfileResponse::Errors(mut e) => Err(e.swap_remove(0).into()),
            })
        }),
    )
}
