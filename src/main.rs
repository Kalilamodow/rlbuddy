#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod common;
mod discord;
mod hotkey;
mod rocket_league;
mod spotify;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    let gui_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "rlbuddy (Not connected)",
        gui_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ui::RlBuddyApp::new(cc)))
        }),
    )
}
