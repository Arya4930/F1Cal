// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod calendar;
mod config;
mod fmt;
mod openf1;
mod theme;
mod track;
mod ui;
mod worker;

use app::Widget;
use config::{level, load_config};
use eframe::egui::ViewportBuilder;

pub const WIDTH: f32 = 480.0;
pub const START_H: f32 = 300.0;

fn main() -> eframe::Result {
    let cfg = load_config();

    let viewport = ViewportBuilder::default()
        .with_inner_size([WIDTH, START_H])
        .with_position([cfg.x, cfg.y])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_taskbar(false)
        .with_window_level(level(cfg.on_bottom));

    eframe::run_native(
        "Desktop Widget",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(Widget::new(cc, cfg)))),
    )
}
