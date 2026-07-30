use crate::{
    auto_setup::AutoSetupWidget,
    discord,
    hotkey::{HotkeyService, HotkeySettings},
    matches::{CurrentMatchWidget, MatchesService, PastMatchesWidget},
    player_info::{PlayerInfoService, PlayerInfoServiceCommand, PlayerSearchWidget},
    settings::SettingsWidget,
    spotify::{SpotifySavedata, SpotifyService, SpotifyWidget},
    stats_api::{RLEvent, StatsApi},
};

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum Panel {
    CurrentMatch,
    PastMatches,
    Discord,
    Spotify,
    PlayerSearch,
    AutoSetup,
    Settings,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::CurrentMatch => "Lobby",
                Panel::PastMatches => "History",
                Panel::Discord => "Discord",
                Panel::Spotify => "Spotify",
                Panel::PlayerSearch => "Player Search",
                Panel::AutoSetup => "Stats API Setup",
                Panel::Settings => "Settings",
            }
        )
    }
}

const OPENABLE_PANELS: [Panel; 7] = [
    Panel::CurrentMatch,
    Panel::Discord,
    Panel::Spotify,
    Panel::PastMatches,
    Panel::AutoSetup,
    Panel::PlayerSearch,
    Panel::Settings,
];

fn visuals_with_transparency(visuals: &mut egui::Visuals, transparency: u8) {
    visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(
        visuals.panel_fill.r(),
        visuals.panel_fill.g(),
        visuals.panel_fill.b(),
        255 - transparency,
    );
}

#[derive(Debug, Serialize, Deserialize)]
struct AppData {
    transparency: u8,
    hotkey_settings: Option<HotkeySettings>,
    rich_presence_settings: Option<discord::DiscordSettings>,
    spotify_data: Option<SpotifySavedata>,
    open_panels: Vec<Panel>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            transparency: 25,
            hotkey_settings: None,
            rich_presence_settings: None,
            spotify_data: None,
            open_panels: vec![Panel::CurrentMatch],
        }
    }
}

pub struct RlBuddyApp {
    current_transparency: Rc<RefCell<u8>>,

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

    player_info_service: PlayerInfoService,
    player_search_widget: PlayerSearchWidget,

    hotkey_service: HotkeyService,
    auto_setup_widget: AutoSetupWidget,
    settings_widget: SettingsWidget,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        egui_system_fonts::set_auto(&ctx, egui_system_fonts::FontStyle::Sans);

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

        let current_transparency = Rc::new(RefCell::new(app_data.transparency));
        RlBuddyApp {
            settings_widget: SettingsWidget::new(&hotkey_service, Rc::clone(&current_transparency)),

            current_transparency,
            prev_hide_pos: None,

            stats_api,

            discord_widget: discord::DiscordWidget::new(
                discord_service.settings_handle(),
                discord_service.state_handle(),
            ),
            discord_service,

            spotify_widget: SpotifyWidget::new(
                spotify_service.state_handle(),
                spotify_service.settings_handle(),
            ),
            spotify_service,

            current_match: CurrentMatchWidget::new(matches_service.state_handle()),
            past_matches: PastMatchesWidget::new(matches_service.state_handle()),
            matches_service,

            hotkey_service,
            player_info_service: PlayerInfoService::new(ctx.clone()),
            player_search_widget: PlayerSearchWidget::new(),

            auto_setup_widget: AutoSetupWidget::new(),
            open_panels: app_data.open_panels,
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
            transparency: *self.current_transparency.borrow(),
            hotkey_settings: Some(self.hotkey_service.settings_handle().read().clone()),
            rich_presence_settings: Some(self.discord_service.settings_handle().read().clone()),
            spotify_data: Some(self.spotify_service.save()),
            open_panels: self.open_panels.clone(),
        };
        eframe::set_value(storage, eframe::APP_KEY, &data);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        visuals_with_transparency(ui.visuals_mut(), *self.current_transparency.borrow());

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
                    let mut to_swap: Option<(usize, usize)> = None; // index, move to
                    let mut to_close: Option<Panel> = None;

                    for (index, panel) in self.open_panels.iter().enumerate() {
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
                                Panel::CurrentMatch => ui.add(&mut self.current_match),
                                Panel::Discord => ui.add(&mut self.discord_widget),
                                Panel::Spotify => ui.add(&mut self.spotify_widget),
                                Panel::PastMatches => ui.add(&mut self.past_matches),
                                Panel::PlayerSearch => ui.add(&mut self.player_search_widget),
                                Panel::AutoSetup => ui.add(&mut self.auto_setup_widget),
                                Panel::Settings => ui.add(&mut self.settings_widget),
                            };
                        });

                        ui.add_space(4.0);
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

        ui.add(&mut self.player_info_service);
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let stats_api_latest = Arc::new(self.stats_api.update());
        self.spotify_service
            .update(&stats_api_latest, self.spotify_widget.get_command());
        self.matches_service.update(ctx, &stats_api_latest);

        let open_player_info: Option<PlayerInfoServiceCommand> = self
            .player_search_widget
            .get_command()
            .or_else(|| self.current_match.get_command())
            .or_else(|| self.past_matches.get_command());

        self.player_info_service.update(open_player_info);
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

        ctx.request_repaint_after(Duration::from_millis(10));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
