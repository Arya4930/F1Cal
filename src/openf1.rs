use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::time::Duration;

use crate::calendar::{Calendar, Meeting, Session};

#[derive(Deserialize)]
pub struct RawSession {
    #[serde(default)]
    session_name: String,
    #[serde(default)]
    session_type: String,
    date_start: String,
    date_end: String,
    #[serde(default)]
    meeting_key: i64,
    #[serde(default)]
    country_name: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    circuit_short_name: String,
    #[serde(default)]
    is_cancelled: bool,
}

pub fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

pub fn fetch_calendar(year: i32) -> Result<Calendar, String> {
    let url = format!("https://api.openf1.org/v1/sessions?year={year}");
    let body = match ureq::get(&url).timeout(Duration::from_secs(20)).call() {
        Ok(r) => r.into_string().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(404, _)) => "[]".to_string(), // OpenF1 uses 404 for "no results"
        Err(e) => return Err(format!("Network error: {e}")),
    };
    let raw: Vec<RawSession> =
        serde_json::from_str(&body).map_err(|e| format!("Bad API response: {e}"))?;

    let mut parsed: Vec<(RawSession, DateTime<Utc>, DateTime<Utc>)> = raw
        .into_iter()
        .filter_map(|r| {
            let (s, e) = (parse_ts(&r.date_start)?, parse_ts(&r.date_end)?);
            Some((r, s, e))
        })
        .collect();
    if parsed.is_empty() {
        return Err(format!("No sessions published for {year}"));
    }
    parsed.sort_by_key(|(_, s, _)| *s);

    let mut meetings: Vec<Meeting> = vec![];
    for (r, start, end) in parsed {
        let session = Session {
            name: r.session_name,
            kind: r.session_type,
            start,
            end,
            cancelled: r.is_cancelled,
        };
        if let Some(m) = meetings.iter_mut().find(|m| m.key == r.meeting_key) {
            m.sessions.push(session);
        } else {
            meetings.push(Meeting {
                key: r.meeting_key,
                country: r.country_name,
                circuit: r.circuit_short_name,
                location: r.location,
                testing: false,
                cancelled: false,
                sessions: vec![session],
            });
        }
    }
    for m in &mut meetings {
        let race = m.sessions.iter().find(|s| s.name == "Race");
        m.testing = race.is_none();
        m.cancelled = match race {
            Some(r) => r.cancelled,
            None => m.sessions.iter().all(|s| s.cancelled),
        };
    }
    meetings.sort_by_key(|m| m.start());
    Ok(Calendar {
        year,
        fetched_at: Utc::now(),
        meetings,
    })
}