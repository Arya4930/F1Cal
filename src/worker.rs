use crate::calendar::{Calendar, load_cache, save_cache};
use crate::openf1::fetch_calendar;
use eframe::egui::self;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Clone, Default)]
pub struct State {
    pub calendar: Option<Calendar>,
    pub loading: bool,
    pub error: Option<String>,
}
pub type Shared = Arc<Mutex<State>>;

/// Background thread: fetch on request, then refresh every 6 h. Always writes the JSON cache.
pub fn spawn_worker(ctx: egui::Context, shared: Shared) -> mpsc::Sender<i32> {
    let (tx, rx) = mpsc::channel::<i32>();
    thread::spawn(move || {
        let mut year: Option<i32> = None;
        loop {
            match rx.recv_timeout(Duration::from_secs(6 * 3600)) {
                Ok(y) => {
                    let mut latest = y;
                    while let Ok(y2) = rx.try_recv() {
                        latest = y2;
                    }
                    year = Some(latest);
                    // Show the cached calendar for that year instantly.
                    let mut s = shared.lock().unwrap();
                    s.calendar = load_cache(latest);
                    s.loading = true;
                    s.error = None;
                    drop(s);
                    ctx.request_repaint();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if let Some(y) = year {
                let res = fetch_calendar(y);
                let mut s = shared.lock().unwrap();
                s.loading = false;
                match res {
                    Ok(cal) => {
                        save_cache(&cal);
                        s.calendar = Some(cal);
                        s.error = None;
                    }
                    Err(e) => s.error = Some(e), // keep the cached calendar on failure
                }
                drop(s);
                ctx.request_repaint();
            }
        }
    });
    tx
}