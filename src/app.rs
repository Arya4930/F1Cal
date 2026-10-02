use chrono::{Local, Utc};
use eframe::egui::{
    self, Align, Color32, Frame, Id, Layout, Margin, RichText, Rounding, Sense, ViewportCommand,
    ViewportId,
};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use crate::calendar::*;
use crate::config::*;
use crate::fmt::*;
use crate::theme::*;
use crate::track::*;
use crate::updater::*;
use crate::worker::*;
use crate::{START_H, WIDTH};

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Appearance,
    Calendar,
}

pub struct Widget {
    pub cfg: Config,
    pub settings_open: bool,
    pub tab: Tab,
    pub scroll_pending: bool,
    pub shared: Shared,
    pub tx: mpsc::Sender<i32>,
    pub last_year: i32,
    pub win_h: f32,
    pub circuits: Vec<Circuit>,
    pub track_key: Option<i64>,
    pub track: Option<Track>,

    pub update: UpdateState,
    pub update_tx: mpsc::Sender<UpdateMsg>,
    pub update_rx: mpsc::Receiver<UpdateMsg>,
}

impl Widget {
    pub fn new(cc: &eframe::CreationContext<'_>, cfg: Config) -> Self {
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = Color32::TRANSPARENT;
        v.extreme_bg_color = Color32::TRANSPARENT;
        v.window_shadow = egui::epaint::Shadow::NONE;
        v.popup_shadow = egui::epaint::Shadow::NONE;
        cc.egui_ctx.set_visuals(v);
        setup_fonts(&cc.egui_ctx);

        let (update_tx, update_rx) = mpsc::channel();
        {
            let tx = update_tx.clone();
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let msg = match check_for_update() {
                    Ok(Some(v)) => UpdateMsg::Available(v),
                    Ok(None) => UpdateMsg::UpToDate,
                    Err(e) => UpdateMsg::Failed(e.to_string()),
                };
                let _ = tx.send(msg);
                ctx.request_repaint();
            });
        }

        // Cached calendar is available immediately (works offline); the worker refreshes it.
        let shared: Shared = Arc::new(Mutex::new(State {
            calendar: load_cache(cfg.year),
            loading: true,
            error: None,
        }));
        let tx = spawn_worker(cc.egui_ctx.clone(), shared.clone());
        let _ = tx.send(cfg.year);

        Self {
            last_year: cfg.year,
            cfg,
            settings_open: false,
            tab: Tab::Appearance,
            scroll_pending: false,
            shared,
            tx,
            win_h: START_H,
            circuits: load_circuits(),
            track_key: None,
            track: None,

            update: UpdateState::Idle,
            update_tx,
            update_rx,
        }
    }

    pub fn refetch(&mut self, force: bool) {
        if force || self.cfg.year != self.last_year {
            self.last_year = self.cfg.year;
            let _ = self.tx.send(self.cfg.year);
        }
    }

    fn card_color(&self) -> Color32 {
        let [r, g, b] = self.cfg.bg;
        Color32::from_rgba_unmultiplied(
            r,
            g,
            b,
            (self.cfg.opacity as f32 / 100.0 * 255.0).round() as u8,
        )
    }

    fn text_color(&self) -> Color32 {
        let [r, g, b] = self.cfg.text;
        Color32::from_rgb(r, g, b)
    }

    fn ensure_track(&mut self, m: &Meeting, year: i32) {
        if self.track_key == Some(m.key) {
            return;
        }
        self.track_key = Some(m.key);
        self.track = if RANDOM_TRACK && !self.circuits.is_empty() {
            let n = self.circuits.len();
            let start = (mix(m.key) % n as u64) as usize;
            // Walk from the random start until one has a loadable SVG.
            (0..n)
                .map(|i| &self.circuits[(start + i) % n])
                .find_map(|c| pick_layout(c, year).and_then(|l| load_track(&l.layout_id)))
        } else {
            find_circuit(&self.circuits, m)
                .and_then(|c| pick_layout(c, year))
                .and_then(|l| load_track(&l.layout_id))
        };
    }
}

impl eframe::App for Widget {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_secs(1));

        while let Ok(msg) = self.update_rx.try_recv() {
            self.update = match msg {
                UpdateMsg::Available(v) => UpdateState::Available(v),
                UpdateMsg::Installed(v) => UpdateState::Done(v),
                UpdateMsg::Failed(e) => {
                    if self.update == UpdateState::Downloading {
                        UpdateState::Failed(e)
                    } else {
                        UpdateState::Idle
                    }
                }
                UpdateMsg::UpToDate => UpdateState::Idle,
            }
        }
        let upd = self.update.clone();
        let mut start_install = false;
        let mut dismiss_update = false;
        let mut restart = false;

        if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
            if ctx.viewport_id() == ViewportId::ROOT {
                self.cfg.x = r.min.x;
                self.cfg.y = r.min.y;
            }
        }

        let now = Utc::now();
        let time_fmt = if self.cfg.use_24h {
            "%H:%M"
        } else {
            "%-I:%M %p"
        };
        let mut open_settings = false;
        let text = self.text_color();
        let dim = text.gamma_multiply(0.6);
        let show = self.cfg.show_schedule;
        let show_track = self.cfg.show_track;
        let st = self.shared.lock().unwrap().clone();
        let next = st
            .calendar
            .as_ref()
            .and_then(|c| c.next_race(now).map(|m| (c, m)));

        if let (Some((cal, m)), true) = (&next, show && show_track) {
            self.ensure_track(m, cal.year);
        }

        let used = egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(self.card_color())
                    .rounding(Rounding::same(18.0))
                    .inner_margin(Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                let bg = ui.interact(ui.max_rect(), Id::new("drag"), Sense::click_and_drag());
                if bg.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                bg.context_menu(|ui| {
                    if ui.button("⚙  Settings").clicked() {
                        open_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Close widget").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });

                ui.spacing_mut().item_spacing.x = 0.0;

                match &upd {
                    UpdateState::Available(v) => {
                        ui.label(
                            RichText::new(format!("Update v{v} available. Download now?"))
                                .size(13.0)
                                .color(text),
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Yes").clicked() {
                                start_install = true;
                            }
                            if ui.button("Later").clicked() {
                                dismiss_update = true;
                            }
                        });
                        ui.add_space(10.0);
                    }
                    UpdateState::Downloading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Downloading update…").size(13.0).color(dim));
                        });
                        ui.add_space(10.0);
                    }
                    UpdateState::Done(v) => {
                        ui.label(
                            RichText::new(format!("Updated to v{v}."))
                                .size(13.0)
                                .color(GREEN),
                        );
                        if ui.button("Restart now").clicked() {
                            restart = true;
                        }
                        ui.add_space(10.0);
                    }
                    UpdateState::Failed(e) => {
                        ui.label(
                            RichText::new(format!("Update failed: {e}"))
                                .size(12.0)
                                .color(RED),
                        );
                        if ui.button("Dismiss").clicked() {
                            dismiss_update = true;
                        }
                        ui.add_space(10.0);
                    }
                    UpdateState::Idle => {}
                }

                ui.columns(2, |cols| {
                    // ---------------- LEFT: sessions grouped by day ----------------
                    {
                        let ui = &mut cols[0];

                        if let (true, Some((_, m))) = (show, &next) {
                            let accent = accent_for(m.key);
                            let mut last_day = None;
                            for s in m.sessions.iter().filter(|s| !s.cancelled) {
                                let ls = s.start.with_timezone(&Local);
                                let day = ls.date_naive();
                                if last_day != Some(day) {
                                    if last_day.is_some() {
                                        ui.add_space(10.0);
                                    }
                                    ui.label(
                                        RichText::new(ls.format("%A").to_string().to_uppercase())
                                            .size(12.0)
                                            .strong()
                                            .color(accent),
                                    );
                                    ui.add_space(2.0);
                                    last_day = Some(day);
                                }

                                let past = s.end <= now;
                                let live = s.start <= now && now < s.end;
                                let name_col = if past { dim } else { text };
                                let time_col = if live {
                                    GREEN
                                } else if past {
                                    dim.gamma_multiply(0.7)
                                } else {
                                    dim
                                };
                                let range = format!(
                                    "{} - {}",
                                    ls.format(time_fmt),
                                    s.end.with_timezone(&Local).format(time_fmt)
                                );

                                ui.horizontal(|ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(40.0, 10.0),
                                        Layout::top_down(Align::Center),
                                        |ui| {
                                            ui.set_min_width(40.0);
                                            ui.spacing_mut().item_spacing.y = 0.0;
                                            ui.label(
                                                RichText::new(ls.format("%d").to_string())
                                                    .size(26.0)
                                                    .strong()
                                                    .color(name_col),
                                            );
                                            ui.label(
                                                RichText::new(
                                                    ls.format("%a").to_string().to_uppercase(),
                                                )
                                                .size(11.0)
                                                .color(dim),
                                            );
                                        },
                                    );
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(&s.name)
                                                .size(16.0)
                                                .strong()
                                                .color(name_col),
                                        );
                                        ui.label(RichText::new(range).size(12.0).color(time_col));
                                    });
                                });
                            }
                        }
                    }

                    // ---------------- RIGHT: header, countdown, track ----------------
                    {
                        let ui = &mut cols[1];
                        if !show {
                            return;
                        }
                        match (&st.calendar, &next) {
                            (None, _) => {
                                let msg = match &st.error {
                                    Some(e) => RichText::new(e.clone()).size(12.0).color(RED),
                                    None => {
                                        RichText::new("Loading calendar…").size(12.0).color(dim)
                                    }
                                };
                                ui.label(msg);
                            }
                            (Some(cal), None) => {
                                ui.label(
                                    RichText::new(format!("No upcoming races in {}", cal.year))
                                        .size(12.0)
                                        .color(dim),
                                );
                            }
                            (Some(_), Some((cal, m))) => {
                                let accent = accent_for(m.key);

                                // Header: country / city / dates, round number on the right
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.label(
                                            RichText::new(m.country.to_uppercase())
                                                .size(22.0)
                                                .strong()
                                                .color(text),
                                        );
                                        ui.label(
                                            RichText::new(m.location.to_uppercase())
                                                .size(14.0)
                                                .strong()
                                                .color(accent),
                                        );
                                        ui.label(
                                            RichText::new(fmt_range_pad(m.start(), m.end()))
                                                .size(13.0)
                                                .color(dim),
                                        );
                                    });
                                    if let Some(r) = cal.round_of(m.key) {
                                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                            ui.label(
                                                RichText::new(r.to_string())
                                                    .size(38.0)
                                                    .strong()
                                                    .color(text.gamma_multiply(0.3)),
                                            );
                                        });
                                    }
                                });

                                // Countdown to the next session (or LIVE)
                                ui.add_space(16.0);
                                let live = m
                                    .sessions
                                    .iter()
                                    .any(|s| !s.cancelled && s.start <= now && now < s.end);
                                let target = m
                                    .sessions
                                    .iter()
                                    .filter(|s| !s.cancelled && s.start > now)
                                    .map(|s| s.start)
                                    .min();
                                if live {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new("LIVE").size(34.0).strong().color(GREEN),
                                        );
                                    });
                                } else if let Some(t) = target {
                                    let secs = (t - now).num_seconds().max(0);
                                    let vals = [
                                        (secs / 86_400, "Days"),
                                        (secs % 86_400 / 3600, "Hrs"),
                                        (secs % 3600 / 60, "Mins"),
                                    ];
                                    ui.columns(3, |c| {
                                        for (i, (v, l)) in vals.iter().enumerate() {
                                            c[i].vertical_centered(|ui| {
                                                ui.spacing_mut().item_spacing.y = 0.0;
                                                ui.label(
                                                    RichText::new(format!("{v:02}"))
                                                        .size(30.0)
                                                        .strong()
                                                        .color(text),
                                                );
                                                ui.label(RichText::new(*l).size(11.0).color(dim));
                                            });
                                        }
                                    });
                                } else {
                                    ui.vertical_centered(|ui| {
                                        ui.label(RichText::new("Weekend finished").color(dim));
                                    });
                                }

                                // Track map + circuit name
                                ui.add_space(14.0);
                                if show_track {
                                    if let Some(t) = &self.track {
                                        draw_track(ui, t, accent);
                                        ui.add_space(10.0);
                                    }
                                }
                                if st.error.is_some() {
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new("Offline · showing saved calendar")
                                            .size(10.0)
                                            .color(dim),
                                    );
                                }
                            }
                        }
                    }
                });
                ui.min_rect().height()
            })
            .inner;

        // Fit the window to its content (card margins = 2 * 18).
        let want = (used + 36.0).max(60.0);
        if (want - self.win_h).abs() > 1.0 {
            self.win_h = want;
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(WIDTH, want)));
            ctx.request_repaint();
        }

        if open_settings {
            if !self.settings_open {
                self.scroll_pending = true;
            }
            self.settings_open = true;
        }

        if dismiss_update {
            self.update = UpdateState::Idle;
        }
        if start_install {
            self.update = UpdateState::Downloading;
            let tx = self.update_tx.clone();
            let ctx2 = ctx.clone();
            std::thread::spawn(move || {
                let msg = match install_update() {
                    Ok(v) => UpdateMsg::Installed(v),
                    Err(e) => UpdateMsg::Failed(e.to_string()),
                };
                let _ = tx.send(msg);
                ctx2.request_repaint();
            });
        }
        if restart {
            save_config(&self.cfg);
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe).spawn();
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        if self.settings_open {
            self.settings_window(ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        save_config(&self.cfg);
    }
}
