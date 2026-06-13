#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod rl_stats_api;

use eframe::egui;

fn main() -> eframe::Result {
    let gui_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([250.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "lobby ranks",
        gui_options,
        Box::new(|_cx| Ok(Box::new(app::RankDisplayApp::new()))),
    )
}
