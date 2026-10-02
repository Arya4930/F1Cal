use chrono::{Datelike, Local};
use eframe::egui::WindowLevel;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
};

pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("DesktopWidgetRs"))
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub x: f32,
    pub y: f32,
    pub bg: [u8; 3],
    pub text: [u8; 3],
    pub opacity: u8, // 0..=100 (%)
    pub use_24h: bool,
    pub on_bottom: bool,
    pub year: i32,
    pub show_schedule: bool,
    pub show_track: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x: 160.0,
            y: 24.0,
            bg: [0x1E, 0x1E, 0x2E],
            text: [0xFF, 0xFF, 0xFF],
            opacity: 80,
            use_24h: true,
            on_bottom: true,
            year: Local::now().year(),
            show_schedule: true,
            show_track: true,
        }
    }
}

pub fn load_config() -> Config {
    data_dir()
        .and_then(|d| fs::read_to_string(d.join("settings.json")).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(c: &Config) {
    if let Some(d) = data_dir() {
        let _ = fs::create_dir_all(&d);
        if let Ok(json) = serde_json::to_string_pretty(c) {
            let _ = fs::write(d.join("settings.json"), json);
        }
    }
}

pub fn level(on_bottom: bool) -> WindowLevel {
    if on_bottom {
        WindowLevel::AlwaysOnBottom
    } else {
        WindowLevel::Normal
    }
}