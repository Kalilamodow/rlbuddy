use crate::{
    auto_setup::AutoSetupWidget,
    discord,
    hotkey::{HotkeyService, HotkeySettings},
    rocket_league::{CurrentMatchWidget, MatchesService, PastMatchesWidget, RLEvent, StatsApi},
    settings::SettingsWidget,
    spotify::{SpotifySavedata, SpotifyService, SpotifyWidget},
};

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    PastMatches,
    Discord,
    Spotify,
    AutoSetup,
    Settings,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::PastMatches => "History",
                Panel::Discord => "Discord",
                Panel::Spotify => "Spotify",
                Panel::AutoSetup => "Stats API Setup",
                Panel::Settings => "Settings",
            }
        )
    }
}

// note: this determines order
const OPENABLE_PANELS: [Panel; 5] = [
    Panel::Discord,
    Panel::Spotify,
    Panel::PastMatches,
    Panel::AutoSetup,
    Panel::Settings,
];

#[derive(Debug, Default, Serialize, Deserialize)]
struct AppData {
    hotkey_settings: Option<HotkeySettings>,
    rich_presence_settings: Option<discord::DiscordSettings>,
    spotify_data: Option<SpotifySavedata>,
}

pub struct RlBuddyApp {
    prev_hide_pos: Option<egui::Pos2>,
    open_panels: Vec<Panel>,

    stats_api: StatsApi,

    spotify_service: SpotifyService,
    spotify_widget: SpotifyWidget,

    discord_service: discord::DiscordService,
    discord_widget: discord::DiscordWidget,

    matches_service: MatchesService,
    current_match: CurrentMatchWidget,
    past_matches: PastMatchesWidget,

    hotkey_service: HotkeyService,
    auto_setup_widget: AutoSetupWidget,
    settings_widget: SettingsWidget,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();

        let app_data = if let Some(storage) = cc.storage
            && let Some(existing_state) = eframe::get_value::<AppData>(storage, eframe::APP_KEY)
        {
            existing_state
        } else {
            AppData::default()
        };

        let stats_api = StatsApi::new();
        let matches_service = MatchesService::new(&ctx);
        let spotify_service = SpotifyService::new(app_data.spotify_data);
        let discord_service = discord::DiscordService::new(
            app_data.rich_presence_settings,
            matches_service.state_handle(),
        );
        let hotkey_service = HotkeyService::new(app_data.hotkey_settings);

        RlBuddyApp {
            prev_hide_pos: None,

            stats_api,
            spotify_service,

            discord_widget: discord::DiscordWidget::new(
                discord_service.settings_handle(),
                discord_service.state_handle(),
            ),
            discord_service,

            current_match: CurrentMatchWidget::new(matches_service.state_handle()),
            past_matches: PastMatchesWidget::new(matches_service.state_handle()),
            matches_service,

            settings_widget: SettingsWidget::new(&hotkey_service),
            hotkey_service,

            open_panels: Vec::new(),
            spotify_widget: SpotifyWidget::new(),

            auto_setup_widget: AutoSetupWidget::new(),
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
}

impl eframe::App for RlBuddyApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let data = AppData {
            hotkey_settings: Some(self.hotkey_service.settings_handle().read().clone()),
            rich_presence_settings: Some(self.discord_service.settings_handle().read().clone()),
            spotify_data: Some(self.spotify_service.save()),
        };
        eframe::set_value(storage, eframe::APP_KEY, &data);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            egui::ComboBox::from_label("")
                .selected_text("Widgets")
                .show_ui(ui, |ui| {
                    for panel in OPENABLE_PANELS {
                        let open = self.open_panels.contains(&panel);

                        if ui.selectable_label(open, panel.to_string()).clicked() {
                            if open {
                                self.open_panels.retain(|p| p != &panel);
                            } else {
                                self.open_panels.push(panel);
                            }
                        }
                    }
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.add(&self.current_match);

                    let mut to_swap: Option<(usize, usize)> = None; // index, move to
                    let mut to_close: Option<Panel> = None;

                    for (index, panel) in self.open_panels.iter().enumerate() {
                        ui.add_space(4.0);

                        let frame =
                            egui::Frame::group(ui.style()).fill(ui.style().visuals.faint_bg_color);

                        frame.show(ui, |ui| {
                            ui.columns_const(|[c1, c2]| {
                                c1.label(egui::RichText::new(panel.to_string()).strong());

                                c2.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |c2| {
                                        if c2.small_button("X").clicked() {
                                            to_close = Some(*panel);
                                        }

                                        c2.add_enabled_ui(
                                            index != self.open_panels.len() - 1,
                                            |c2| {
                                                if c2.small_button("\\/").clicked() {
                                                    to_swap = Some((index, index + 1));
                                                }
                                            },
                                        );

                                        c2.add_enabled_ui(index != 0, |c2| {
                                            if c2.small_button("/\\").clicked() {
                                                to_swap = Some((index, index - 1));
                                            }
                                        });
                                    },
                                );
                            });

                            ui.separator();

                            match panel {
                                Panel::Discord => ui.add(&mut self.discord_widget),
                                Panel::Spotify => ui.add(&mut self.spotify_widget),
                                Panel::PastMatches => ui.add(&self.past_matches),
                                Panel::AutoSetup => ui.add(&mut self.auto_setup_widget),
                                Panel::Settings => ui.add(&mut self.settings_widget),
                            };
                        });
                    }

                    if let Some(to_close) = to_close {
                        self.open_panels.retain(|p| p != &to_close);
                    }
                    if let Some(to_shift) = to_swap {
                        self.open_panels.swap(to_shift.0, to_shift.1);
                    }
                })
            });
        });
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let stats_api_latest = Arc::new(self.stats_api.update());
        let spotify_latest = self
            .spotify_service
            .update(&stats_api_latest, self.spotify_widget.get_command());
        self.matches_service.update(ctx, &stats_api_latest);

        self.discord_service.update();

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

        if let Some(should_overlay) = self.hotkey_service.update() {
            if should_overlay {
                self.show(ctx);
            } else {
                self.hide(ctx);
            }
        }

        self.spotify_widget.logic(spotify_latest);

        ctx.request_repaint_after(Duration::from_millis(10));
    }
}
