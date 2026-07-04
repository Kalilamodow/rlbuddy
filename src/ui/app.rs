use crate::ui::settings::{SettingsState, SettingsWidget};

use super::{hotkey, matches::Matches};
use eframe::egui;
use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;

fn bold_text(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).strong()
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    Matches,
    Settings,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::Matches => "Matches",
                Panel::Settings => "Settings",
            }
        )
    }
}

const ALL_PANELS: [Panel; 2] = [Panel::Matches, Panel::Settings];

pub struct RlBuddyApp {
    error_receiver: mpsc::Receiver<String>,
    current_error: Option<String>,
    prev_hide_pos: Option<egui::Pos2>,
    overlay_rx: mpsc::Receiver<bool>,

    open_panels: HashSet<Panel>,
    matches: Matches,
    settings: SettingsWidget,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (errors_tx, errors_rx) = mpsc::channel();
        let (overlay_tx, overlay_rx) = mpsc::channel();

        let settings = if let Some(storage) = cc.storage
            && let Some(existing_state) =
                eframe::get_value::<SettingsState>(storage, eframe::APP_KEY)
        {
            SettingsWidget::from_existing(existing_state)
        } else {
            SettingsWidget::default()
        };

        let overlay_tx_for_hotkey = overlay_tx.clone();
        let ctx_for_hotkey = ctx.clone();
        let settings_for_hotkey = settings.clone_state();

        thread::spawn(move || {
            hotkey::listen_for_hotkey(overlay_tx_for_hotkey, ctx_for_hotkey, settings_for_hotkey);
        });

        RlBuddyApp {
            error_receiver: errors_rx,
            current_error: None,
            overlay_rx,
            prev_hide_pos: None,

            open_panels: HashSet::from([Panel::Matches]),
            matches: Matches::new(&ctx, overlay_tx.clone(), errors_tx),
            settings,
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
        eframe::set_value(storage, eframe::APP_KEY, &self.settings.clone_state());
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
                                Panel::Settings => ui.add(&self.settings),
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
