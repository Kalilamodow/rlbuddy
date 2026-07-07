use super::{
    discord::{DiscordWidget, RichPresenceSettings},
    hotkey::{HotkeySettings, HotkeyWidget},
    matches::Matches,
    spotify,
};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::{collections::HashSet, rc::Rc, sync::Mutex};

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    Matches,
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
                Panel::Matches => "Matches",
                Panel::HotkeySettings => "Hotkey",
                Panel::DiscordSettings => "Discord",
                Panel::Spotify => "Spotify",
            }
        )
    }
}

const ALL_PANELS: [Panel; 4] = [
    Panel::Matches,
    Panel::HotkeySettings,
    Panel::DiscordSettings,
    Panel::Spotify,
];

#[derive(Debug, Default, Serialize, Deserialize)]
struct AppData {
    hotkey_settings: Option<HotkeySettings>,
    rich_presence_settings: Option<RichPresenceSettings>,
    spotify_data: Option<spotify::SpotifySavedata>,
}

pub struct RlBuddyApp {
    error_receiver: mpsc::Receiver<String>,
    current_error: Option<String>,
    prev_hide_pos: Option<egui::Pos2>,
    overlay_rx: mpsc::Receiver<bool>,

    open_panels: HashSet<Panel>,
    matches: Matches,
    hotkey_settings: HotkeyWidget,
    discord: DiscordWidget,
    spotify: spotify::SpotifyWidget,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (errors_tx, errors_rx) = mpsc::channel();
        let (overlay_tx, overlay_rx) = mpsc::channel();

        let app_data = if let Some(storage) = cc.storage
            && let Some(existing_state) = eframe::get_value::<AppData>(storage, eframe::APP_KEY)
        {
            existing_state
        } else {
            AppData::default()
        };

        let hotkey_widget =
            HotkeyWidget::new(overlay_tx.clone(), ctx.clone(), app_data.hotkey_settings);
        hotkey_widget.start_listening();

        let rich_presence = Rc::new(Mutex::new(crate::discord::RichPresence::new()));

        let spotify_widget = spotify::SpotifyWidget::new(app_data.spotify_data);

        RlBuddyApp {
            error_receiver: errors_rx,
            current_error: None,
            overlay_rx,
            prev_hide_pos: None,

            open_panels: HashSet::from([Panel::Matches]),
            matches: Matches::new(
                &ctx,
                Rc::clone(&rich_presence),
                overlay_tx.clone(),
                errors_tx,
                spotify_widget.cmd(),
            ),
            hotkey_settings: hotkey_widget,
            discord: DiscordWidget::new(Rc::clone(&rich_presence), app_data.rich_presence_settings),
            spotify: spotify_widget,
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
        if let Ok(new_error) = self.error_receiver.try_recv() {
            self.current_error = Some(new_error);
        }

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                for panel in ALL_PANELS {
                    self.panel_add_button(ui, &panel.to_string(), panel);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let connected = self.matches.is_connected();

                    if connected {
                        ui.label(
                            egui::RichText::new("Connected").color(egui::Color32::LIGHT_GREEN),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Not connected").color(egui::Color32::LIGHT_RED),
                        );
                    }
                });
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(err) = &self.current_error {
                ui.label(bold_text("Fatal error"));
                ui.label(err);
                if ui.button("Exit").clicked() {
                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
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
                                Panel::Matches => ui.add(&self.matches),
                                Panel::HotkeySettings => ui.add(&self.hotkey_settings),
                                Panel::DiscordSettings => ui.add(&self.discord),
                                Panel::Spotify => ui.add(&mut self.spotify),
                            };
                        }
                    }
                });
            }
        });
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(should_overlay) = self.overlay_rx.try_recv() {
            if should_overlay {
                self.show(ctx);
            } else {
                self.hide(ctx);
            }
        }

        self.matches.logic(ctx);
    }
}
