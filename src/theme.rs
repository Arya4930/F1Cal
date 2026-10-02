use eframe::egui::{self, Color32};
use std::fs;

pub const BLUE: Color32 = Color32::from_rgb(0x89, 0xB4, 0xFA);
pub const GREEN: Color32 = Color32::from_rgb(0xA6, 0xE3, 0xA1);
pub const RED: Color32 = Color32::from_rgb(0xF3, 0x8B, 0xA8);
pub const GRAY: Color32 = Color32::from_gray(130);

/// Accent colour is picked from this list per race weekend.
pub const ACCENTS: [Color32; 6] = [
    Color32::from_rgb(0x4A, 0x6C, 0xF7), // blue
    Color32::from_rgb(0xF5, 0xC5, 0x42), // yellow
    Color32::from_rgb(0xE5, 0x48, 0x4D), // red
    Color32::from_rgb(0x2D, 0xD4, 0xBF), // teal
    Color32::from_rgb(0xF9, 0x73, 0x16), // orange
    Color32::from_rgb(0xA7, 0x8B, 0xFA), // purple
];

pub fn mix(k: i64) -> u64 {
    let mut z = (k as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn accent_for(key: i64) -> Color32 {
    ACCENTS[(mix(key.wrapping_add(7)) % ACCENTS.len() as u64) as usize]
}

pub fn setup_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};

    let mut fonts = FontDefinitions::default();

    // Semibold becomes the main font for everything.
    if let Ok(data) = fs::read(r"C:\Windows\Fonts\seguisb.ttf") {
        fonts
            .font_data
            .insert("semibold".into(), FontData::from_owned(data));
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "semibold".into());
    }

    // Heavier weight, available as a named family for headings and numbers.
    if let Ok(data) = fs::read(r"C:\Windows\Fonts\segoeuib.ttf") {
        fonts
            .font_data
            .insert("bold".into(), FontData::from_owned(data));
        fonts
            .families
            .insert(FontFamily::Name("bold".into()), vec!["bold".into()]);
    }

    ctx.set_fonts(fonts);
}