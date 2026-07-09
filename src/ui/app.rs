use crate::rocket_league::{RLEvent, StatsApi};

use super::{
    discord::{DiscordWidget, RichPresenceSettings},
    hotkey::{HotkeySettings, HotkeyWidget},
    matches::{CurrentMatch, PastMatchesWidget},
    spotify,
};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use std::{sync::mpsc, time::Duration};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    CurrentMatch,
    PastMatches,
    HotkeySettings,
    DiscordSettings,
    Spotify,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::CurrentMatch => "Match",
                Panel::PastMatches => "History",
                Panel::HotkeySettings => "Keybind",
                Panel::DiscordSettings => "Discord",
                Panel::Spotify => "Spotify",
            }
        )
    }
}

// note: this determines order
const ALL_PANELS: [Panel; 5] = [
    Panel::CurrentMatch,
    Panel::HotkeySettings,
    Panel::DiscordSettings,
    Panel::Spotify,
    Panel::PastMatches,
];

#[derive(Debug, Default, Serialize, Deserialize)]
struct AppData {
    hotkey_settings: Option<HotkeySettings>,
    rich_presence_settings: Option<RichPresenceSettings>,
    spotify_data: Option<spotify::SpotifySavedata>,
}

pub struct RlBuddyApp {
    prev_hide_pos: Option<egui::Pos2>,
    overlay_rx: mpsc::Receiver<bool>,
    open_panels: HashSet<Panel>,

    stats_api: StatsApi,

    current_match: CurrentMatch,
    hotkey_settings: HotkeyWidget,
    discord: DiscordWidget,
    spotify: spotify::SpotifyWidget,
    past_matches: PastMatchesWidget,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (overlay_tx, overlay_rx) = mpsc::channel();

        let app_data = if let Some(storage) = cc.storage
            && let Some(existing_state) = eframe::get_value::<AppData>(storage, eframe::APP_KEY)
        {
            existing_state
        } else {
            AppData::default()
        };

        let stats_api = StatsApi::new();

        let hotkey_widget =
            HotkeyWidget::new(overlay_tx.clone(), ctx.clone(), app_data.hotkey_settings);
        hotkey_widget.start_listening();

        let discord_widget = DiscordWidget::new(app_data.rich_presence_settings);
        let spotify_widget = spotify::SpotifyWidget::new(app_data.spotify_data);
        let past_matches_widget = PastMatchesWidget::new();

        RlBuddyApp {
            overlay_rx,
            prev_hide_pos: None,

            stats_api,

            open_panels: HashSet::from([Panel::CurrentMatch]),
            current_match: CurrentMatch::new(
                &ctx,
                discord_widget.cmd(),
                spotify_widget.cmd(),
                past_matches_widget.cmd(),
            ),
            hotkey_settings: hotkey_widget,
            discord: discord_widget,
            spotify: spotify_widget,
            past_matches: past_matches_widget,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.prev_hide_pos = ctx.input(|i| {
            i.viewport()
                .outer_rect
                .map(|outer_rect| egui::pos2(outer_rect.left(), outer_rect.top()))
        });

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(8.0, 8.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
    }

    fn hide(&self, ctx: &egui::Context) {
        if let Some(move_to) = self.prev_hide_pos {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(move_to));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnBottom,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
    }

    fn panel_add_button(&mut self, ui: &mut egui::Ui, text: &str, panel: Panel) {
        if ui
            .add_enabled(!self.open_panels.contains(&panel), egui::Button::new(text))
            .clicked()
        {
            self.open_panels.insert(panel);
        }
    }

    fn panel_remove_button(&mut self, ui: &mut egui::Ui, text: &str, panel: Panel) {
        if ui.button(text).clicked() {
            self.open_panels.remove(&panel);
        }
    }
}

impl eframe::App for RlBuddyApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let data = AppData {
            hotkey_settings: Some(self.hotkey_settings.get_settings().read().unwrap().clone()),
            rich_presence_settings: Some(self.discord.clone_settings()),
            spotify_data: Some(self.spotify.save()),
        };
        eframe::set_value(storage, eframe::APP_KEY, &data);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                for panel in ALL_PANELS {
                    self.panel_add_button(ui, &panel.to_string(), panel);
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    if self.open_panels.is_empty() {
                        ui.label("No panels open");
                        return;
                    }

                    let mut is_first = true;

                    for panel in ALL_PANELS {
                        if self.open_panels.contains(&panel) {
                            if !is_first {
                                ui.separator();
                            }
                            is_first = false;

                            self.panel_remove_button(ui, &panel.to_string(), panel);
                            match panel {
                                Panel::CurrentMatch => ui.add(&self.current_match),
                                Panel::HotkeySettings => ui.add(&self.hotkey_settings),
                                Panel::DiscordSettings => ui.add(&mut self.discord),
                                Panel::Spotify => ui.add(&mut self.spotify),
                                Panel::PastMatches => ui.add(&mut self.past_matches),
                            };
                        }
                    }
                })
            });
        });
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let stats_api_latest = Arc::new(self.stats_api.update());

        if let Some(event) = stats_api_latest.as_ref() {
            match event {
                RLEvent::Connected => ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                    "rlbuddy (connected)".to_string(),
                )),
                RLEvent::Disconnected => ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                    "rlbuddy (not connected".to_string(),
                )),
                _ => {}
            }
        }

        while let Ok(should_overlay) = self.overlay_rx.try_recv() {
            if should_overlay {
                self.show(ctx);
            } else {
                self.hide(ctx);
            }
        }

        self.current_match.logic(ctx, Arc::clone(&stats_api_latest));

        ctx.request_repaint_after(Duration::from_millis(10));
    }
}
