use chrono::{DateTime, Local, Utc};
use eframe::egui::{
    self, collapsing_header::CollapsingState, Align, Color32, Frame, Layout, Margin,
    RichText, Rounding, ScrollArea, Slider, Stroke, ViewportBuilder, ViewportCommand,
    ViewportId
};

use crate::app::*;
use crate::config::*;
use crate::fmt::fmt_range;
use crate::theme::*;
use crate::calendar::Meeting;

impl Widget {
    pub fn settings_window(&mut self, ctx: &egui::Context) {
        let state = self.shared.lock().unwrap().clone();
        let scroll_pending = self.scroll_pending;
        let cfg = &mut self.cfg;
        let tab = &mut self.tab;
        let (mut close, mut level_changed, mut refetch, mut force, mut scrolled) =
            (false, false, false, false, false);
        let now = Utc::now();

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("settings"),
            ViewportBuilder::default()
                .with_title("Widget settings")
                .with_inner_size([540.0, 640.0])
                .with_min_inner_size([460.0, 420.0]),
            |ctx, _class| {
                egui::CentralPanel::default()
                    .frame(
                        Frame::none()
                            .fill(Color32::from_gray(24))
                            .inner_margin(Margin::same(16.0)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut *tab,
                                Tab::Appearance,
                                RichText::new("Appearance").size(15.0),
                            );
                            ui.selectable_value(
                                &mut *tab,
                                Tab::Calendar,
                                RichText::new("Calendar").size(15.0),
                            );
                        });
                        ui.separator();
                        ui.add_space(6.0);

                        match *tab {
                            Tab::Appearance => {
                                egui::Grid::new("appearance")
                                    .num_columns(2)
                                    .spacing([14.0, 12.0])
                                    .show(ui, |ui| {
                                        ui.label("Background");
                                        ui.color_edit_button_srgb(&mut cfg.bg);
                                        ui.end_row();

                                        ui.label("Text colour");
                                        ui.color_edit_button_srgb(&mut cfg.text);
                                        ui.end_row();

                                        ui.label("Opacity");
                                        ui.add(Slider::new(&mut cfg.opacity, 0..=100).suffix("%"));
                                        ui.end_row();

                                        ui.label("24-hour clock");
                                        ui.checkbox(&mut cfg.use_24h, "");
                                        ui.end_row();

                                        ui.label("Stay on desktop");
                                        level_changed |=
                                            ui.checkbox(&mut cfg.on_bottom, "").changed();
                                        ui.end_row();

                                        ui.label("Next race on widget");
                                        ui.checkbox(&mut cfg.show_schedule, "");
                                        ui.end_row();
                                        ui.label("Track map");
                                        ui.checkbox(&mut cfg.show_track, "");
                                        ui.end_row();
                                    });
                                ui.add_space(16.0);
                                if ui.button("Reset to defaults").clicked() {
                                    *cfg = Config {
                                        x: cfg.x,
                                        y: cfg.y,
                                        ..Config::default()
                                    };
                                    level_changed = true;
                                    refetch = true;
                                }
                            }
                            Tab::Calendar => {
                                ui.horizontal(|ui| {
                                    ui.label("Season");
                                    let y = ui.add(
                                        egui::DragValue::new(&mut cfg.year).range(2023..=2100),
                                    );
                                    refetch |= y.drag_stopped() || y.lost_focus();
                                    if ui.button("⟳ Refresh").clicked() {
                                        force = true;
                                    }
                                    if state.loading {
                                        ui.spinner();
                                    }
                                });
                                if let Some(e) = &state.error {
                                    ui.colored_label(RED, e);
                                }
                                ui.add_space(6.0);

                                match &state.calendar {
                                    None => {
                                        ui.colored_label(
                                            GRAY,
                                            "No calendar saved yet for this season.",
                                        );
                                    }
                                    Some(cal) => {
                                        let local = cal.fetched_at.with_timezone(&Local);
                                        ui.colored_label(
                                            GRAY,
                                            format!(
                                                "{} race weekends · saved {}",
                                                cal.race_count(),
                                                local.format("%-d %b %H:%M")
                                            ),
                                        );
                                        ui.add_space(6.0);
                                        let next_key = cal.next_race(now).map(|m| m.key);
                                        ScrollArea::vertical().auto_shrink([false, false]).show(
                                            ui,
                                            |ui| {
                                                for m in &cal.meetings {
                                                    let is_next = Some(m.key) == next_key;
                                                    let round = cal.round_of(m.key);
                                                    scrolled |= meeting_card(
                                                        ui,
                                                        m,
                                                        round,
                                                        is_next,
                                                        now,
                                                        cfg.use_24h,
                                                        scroll_pending && is_next,
                                                    );
                                                    ui.add_space(6.0);
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    });
                if ctx.input(|i| i.viewport().close_requested()) {
                    close = true;
                }
            },
        );

        if scrolled {
            self.scroll_pending = false;
        }
        if level_changed {
            ctx.send_viewport_cmd_to(
                ViewportId::ROOT,
                ViewportCommand::WindowLevel(level(self.cfg.on_bottom)),
            );
        }
        if refetch || close {
            self.refetch(false);
        }
        if force {
            self.refetch(true);
        }
        if close {
            self.settings_open = false;
        }
        if close || level_changed {
            save_config(&self.cfg);
        }
    }
}

fn meeting_card(
    ui: &mut egui::Ui,
    m: &Meeting,
    round: Option<usize>,
    is_next: bool,
    now: DateTime<Utc>,
    use_24h: bool,
    scroll_to: bool,
) -> bool {
    let (status, status_color) = if m.cancelled {
        ("CANCELLED", RED)
    } else if m.end() < now {
        ("DONE", GRAY)
    } else if m.start() <= now {
        ("LIVE", GREEN)
    } else if is_next {
        ("NEXT", BLUE)
    } else {
        ("", GRAY)
    };
    let tag = if m.testing {
        "TEST".to_string()
    } else {
        round.map_or("—".into(), |r| format!("R{r}"))
    };
    let faded = m.cancelled || m.end() < now;
    let title_color = if faded { GRAY } else { Color32::WHITE };

    let (day_fmt, time_fmt) = if use_24h {
        ("%a %-d %b   %H:%M", "%H:%M")
    } else {
        ("%a %-d %b   %-I:%M %p", "%-I:%M %p")
    };

    let resp = Frame::none()
        .fill(if is_next {
            Color32::from_rgb(0x26, 0x2E, 0x48)
        } else {
            Color32::from_gray(34)
        })
        .stroke(if is_next {
            Stroke::new(1.0 as f32, BLUE)
        } else {
            Stroke::NONE
        })
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let id = ui.make_persistent_id(("meeting", m.key));
            CollapsingState::load_with_default_open(ui.ctx(), id, is_next)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(tag).strong().monospace().color(if is_next {
                        BLUE
                    } else {
                        GRAY
                    }));
                    let mut name = RichText::new(format!("{} · {}", m.country, m.circuit))
                        .strong()
                        .color(title_color);
                    if m.cancelled {
                        name = name.strikethrough();
                    }
                    ui.label(name);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(status)
                                .size(11.0)
                                .strong()
                                .color(status_color),
                        );
                        ui.label(
                            RichText::new(fmt_range(m.start(), m.end()))
                                .size(12.0)
                                .color(GRAY),
                        );
                    });
                })
                .body(|ui| {
                    ui.add_space(4.0);
                    egui::Grid::new(("sessions", m.key))
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            for s in &m.sessions {
                                let col = if s.cancelled {
                                    RED
                                } else if s.end <= now {
                                    GRAY
                                } else {
                                    Color32::from_gray(225)
                                };
                                let mut name = RichText::new(s.name.clone()).color(col);
                                if s.cancelled {
                                    name = name.strikethrough();
                                }
                                ui.label(name);
                                ui.label(
                                    RichText::new(format!(
                                        "{}  –  {}",
                                        s.start.with_timezone(&Local).format(day_fmt),
                                        s.end.with_timezone(&Local).format(time_fmt)
                                    ))
                                    .monospace()
                                    .color(col),
                                );
                                ui.end_row();
                            }
                        });
                });
        });

    if scroll_to {
        resp.response.scroll_to_me(Some(Align::Center));
    }
    scroll_to
}