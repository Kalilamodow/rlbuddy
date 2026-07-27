use std::sync::Arc;

use eframe::egui;

use crate::player_info::trn::{ProfileData, Segment};

pub struct TrackerWidget {
    profile: Arc<ProfileData>,
}

impl TrackerWidget {
    pub fn new(profile: Arc<ProfileData>) -> Self {
        Self { profile }
    }

    fn render_overview(&self, ui: &mut egui::Ui) {
        let Some(overview) = self.profile.segments.iter().find_map(|s| {
            if let Segment::Overview(ov) = s {
                Some(ov)
            } else {
                None
            }
        }) else {
            return;
        };

        ui.label("Overview");
        egui::Grid::new(format!(
            "overview for {}",
            self.profile.platform_info.platform_user_handle
        ))
        .show(ui, |ui| {
            ui.strong(&overview.stats.wins.display_name);
            ui.label(&overview.stats.wins.display_value);
            ui.end_row();

            ui.strong(&overview.stats.goals.display_name);
            ui.label(&overview.stats.goals.display_value);
            ui.end_row();

            ui.strong(&overview.stats.season_reward_level.display_name);
            ui.label(&overview.stats.season_reward_level.metadata.rank_name);
            ui.end_row();
        });
    }
}

impl egui::Widget for TrackerWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            self.render_overview(ui);
        })
        .response
    }
}
