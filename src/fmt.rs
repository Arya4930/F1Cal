use chrono::{DateTime, Datelike, Local, Utc};

pub fn fmt_range_pad(a: DateTime<Utc>, b: DateTime<Utc>) -> String {
    let (a, b) = (a.with_timezone(&Local), b.with_timezone(&Local));
    let s = if a.month() == b.month() {
        if a.day() == b.day() {
            a.format("%d %b").to_string()
        } else {
            format!("{}-{} {}", a.format("%d"), b.format("%d"), a.format("%b"))
        }
    } else {
        format!("{}-{}", a.format("%d %b"), b.format("%d %b"))
    };
    s.to_uppercase()
}

pub fn fmt_range(a: DateTime<Utc>, b: DateTime<Utc>) -> String {
    let (a, b) = (a.with_timezone(&Local), b.with_timezone(&Local));
    if a.month() == b.month() {
        if a.day() == b.day() {
            a.format("%-d %b").to_string()
        } else {
            format!("{}–{} {}", a.day(), b.day(), a.format("%b"))
        }
    } else {
        format!("{} – {}", a.format("%-d %b"), b.format("%-d %b"))
    }
}