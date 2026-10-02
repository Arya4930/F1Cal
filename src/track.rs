use crate::calendar::Meeting;
use eframe::egui::{self, Color32, Sense, Stroke};
use serde::Deserialize;
use std::{fs, path::PathBuf};

pub const TRACK_H: f32 = 150.0;
pub const RANDOM_TRACK: bool = true;

const CIRCUITS_JSON: &str =
    r"C:\Users\Arya\Desktop\Arya\c\rust\rWidget\src\f1-circuits-svg-main\circuits.json";
const TRACKS_DIR: &str =
    r"C:\Users\Arya\Desktop\Arya\c\rust\rWidget\src\f1-circuits-svg-main\tracks";

#[derive(Deserialize)]
pub struct CircuitLayout {
    #[serde(rename = "layoutId")]
    pub layout_id: String,
    #[serde(default)]
    pub seasons: String,
}

#[derive(Deserialize)]
pub struct Circuit {
    id: String,
    name: String,
    #[serde(default)]
    layouts: Vec<CircuitLayout>,
}

pub fn load_circuits() -> Vec<Circuit> {
    fs::read_to_string(CIRCUITS_JSON)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// OpenF1 name (circuit_short_name or location) -> circuits.json id.
/// Verify these ids against your circuits.json; add more as needed.
const ALIASES: &[(&str, &str)] = &[
    ("Melbourne", "albert-park"),
    ("Sakhir", "bahrain"),
    ("Monte Carlo", "monaco"),
    ("Montreal", "montreal"),
    ("Spielberg", "red-bull-ring"),
    ("Singapore", "marina-bay"),
    ("Marina Bay", "marina-bay"),
    ("Yas Marina Circuit", "yas-marina"),
    ("Yas Island", "yas-marina"),
    ("Austin", "americas"),
    ("Mexico City", "rodriguez"),
    ("Sao Paulo", "interlagos"),
    ("Lusail", "lusail"),
    ("Catalunya", "catalunya"),
    ("Spa-Francorchamps", "spa-francorchamps"),
    ("Kuala Lumpur", "sepang"),
];

pub fn find_circuit<'a>(circuits: &'a [Circuit], m: &Meeting) -> Option<&'a Circuit> {
    let keys = [norm(&m.circuit), norm(&m.location)];
    for k in keys.iter().filter(|k| !k.is_empty()) {
        if let Some((_, id)) = ALIASES.iter().find(|(a, _)| norm(a) == *k) {
            if let Some(c) = circuits.iter().find(|c| c.id == *id) {
                return Some(c);
            }
        }
    }
    circuits.iter().find(|c| {
        keys.iter()
            .any(|k| !k.is_empty() && (norm(&c.id) == *k || norm(&c.name).contains(k.as_str())))
    })
}

/// Picks the layout whose `seasons` ("1955,1957,1961-1962") contains `year`, else the newest.
pub fn pick_layout(c: &Circuit, year: i32) -> Option<&CircuitLayout> {
    c.layouts
        .iter()
        .find(|l| {
            l.seasons.split(',').any(|part| {
                let part = part.trim();
                match part.split_once('-') {
                    Some((a, b)) => matches!(
                        (a.trim().parse::<i32>(), b.trim().parse::<i32>()),
                        (Ok(a), Ok(b)) if (a..=b).contains(&year)
                    ),
                    None => part.parse::<i32>().ok() == Some(year),
                }
            })
        })
        .or_else(|| c.layouts.last())
}

pub struct Track {
    strokes: Vec<Vec<egui::Pos2>>, // origin shifted to (0,0)
    size: egui::Vec2,
}

pub fn load_track(layout_id: &str) -> Option<Track> {
    use svgtypes::{SimplePathSegment as S, SimplifyingPathParser};

    let path = PathBuf::from(TRACKS_DIR).join(format!("{layout_id}.svg"));
    let text = fs::read_to_string(path).ok()?;
    let doc = roxmltree::Document::parse(&text).ok()?;

    let mut strokes: Vec<Vec<egui::Pos2>> = vec![];
    for node in doc.descendants().filter(|n| n.has_tag_name("path")) {
        let Some(d) = node.attribute("d") else {
            continue;
        };
        let mut cur: Vec<egui::Pos2> = vec![];
        let mut start = egui::pos2(0.0, 0.0);
        let mut last = start;
        for seg in SimplifyingPathParser::from(d).flatten() {
            match seg {
                S::MoveTo { x, y } => {
                    if cur.len() > 1 {
                        strokes.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                    last = egui::pos2(x as f32, y as f32);
                    start = last;
                    cur.push(last);
                }
                S::LineTo { x, y } => {
                    last = egui::pos2(x as f32, y as f32);
                    cur.push(last);
                }
                S::Quadratic { x1, y1, x, y } => {
                    let (c, e) = (
                        egui::pos2(x1 as f32, y1 as f32),
                        egui::pos2(x as f32, y as f32),
                    );
                    for i in 1..=12 {
                        let t = i as f32 / 12.0;
                        let u = 1.0 - t;
                        cur.push(egui::pos2(
                            u * u * last.x + 2.0 * u * t * c.x + t * t * e.x,
                            u * u * last.y + 2.0 * u * t * c.y + t * t * e.y,
                        ));
                    }
                    last = e;
                }
                S::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    let (c1, c2, e) = (
                        egui::pos2(x1 as f32, y1 as f32),
                        egui::pos2(x2 as f32, y2 as f32),
                        egui::pos2(x as f32, y as f32),
                    );
                    for i in 1..=16 {
                        let t = i as f32 / 16.0;
                        let u = 1.0 - t;
                        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        cur.push(egui::pos2(
                            a * last.x + b * c1.x + c * c2.x + d * e.x,
                            a * last.y + b * c1.y + c * c2.y + d * e.y,
                        ));
                    }
                    last = e;
                }
                S::ClosePath => {
                    cur.push(start);
                    last = start;
                }
            }
        }
        if cur.len() > 1 {
            strokes.push(cur);
        }
    }
    if strokes.is_empty() {
        return None;
    }

    let (mut min, mut max) = (
        egui::pos2(f32::MAX, f32::MAX),
        egui::pos2(f32::MIN, f32::MIN),
    );
    for p in strokes.iter().flatten() {
        min = min.min(*p);
        max = max.max(*p);
    }
    for p in strokes.iter_mut().flatten() {
        *p -= min.to_vec2();
    }
    Some(Track {
        strokes,
        size: (max - min).max(egui::vec2(1.0, 1.0)),
    })
}

pub fn draw_track(ui: &mut egui::Ui, t: &Track, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), TRACK_H),
        Sense::hover(), // hover only, so dragging the widget still works
    );
    let scale = ((rect.width() - 8.0) / t.size.x).min((rect.height() - 8.0) / t.size.y);
    let origin = rect.center() - t.size * scale * 0.5;
    let stroke = Stroke::new(3.0 as f32, color);
    for s in &t.strokes {
        let pts: Vec<_> = s.iter().map(|p| origin + p.to_vec2() * scale).collect();
        ui.painter().add(egui::Shape::line(pts, stroke));
    }
}