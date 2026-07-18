#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod auto_setup;
mod common;
mod discord;
mod hotkey;
mod matches;
mod settings;
mod spotify;
mod stats_api;

use eframe::egui;

fn main() -> eframe::Result {
    let gui_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 600.0])
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "rlbuddy (Not connected)",
        gui_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::RlBuddyApp::new(cc)))
        }),
    )
}
