use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
};

use crate::config::data_dir;

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub name: String,
    pub kind: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub cancelled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Meeting {
    pub key: i64,
    pub country: String,
    pub circuit: String,
    pub location: String,
    pub testing: bool, // no "Race" session => pre-season testing
    pub cancelled: bool,
    pub sessions: Vec<Session>,
}

impl Meeting {
    pub fn start(&self) -> DateTime<Utc> {
        self.sessions.iter().map(|s| s.start).min().unwrap()
    }
    pub fn end(&self) -> DateTime<Utc> {
        self.sessions.iter().map(|s| s.end).max().unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Calendar {
    pub year: i32,
    pub fetched_at: DateTime<Utc>,
    pub meetings: Vec<Meeting>,
}

impl Calendar {
    /// First race weekend that hasn't finished yet (testing and cancelled excluded).
    pub fn next_race(&self, now: DateTime<Utc>) -> Option<&Meeting> {
        self.meetings
            .iter()
            .find(|m| !m.testing && !m.cancelled && m.end() >= now)
    }
    /// Round number counts race weekends only, skipping cancelled ones.
    pub fn round_of(&self, key: i64) -> Option<usize> {
        self.meetings
            .iter()
            .filter(|m| !m.testing && !m.cancelled)
            .position(|m| m.key == key)
            .map(|i| i + 1)
    }
    pub fn race_count(&self) -> usize {
        self.meetings
            .iter()
            .filter(|m| !m.testing && !m.cancelled)
            .count()
    }
}

pub fn cache_path(year: i32) -> Option<PathBuf> {
    data_dir().map(|d| d.join(format!("calendar_{year}.json")))
}

pub fn load_cache(year: i32) -> Option<Calendar> {
    serde_json::from_str(&fs::read_to_string(cache_path(year)?).ok()?).ok()
}

pub fn save_cache(cal: &Calendar) {
    if let (Some(p), Ok(json)) = (cache_path(cal.year), serde_json::to_string_pretty(cal)) {
        if let Some(d) = p.parent() {
            let _ = fs::create_dir_all(d);
        }
        let _ = fs::write(p, json);
    }
}