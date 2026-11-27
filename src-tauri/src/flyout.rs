use crate::{AppState, calculator};
use chrono::{DateTime, Local};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    Emitter, Manager, PhysicalPosition, PhysicalSize, Rect, WebviewUrl, WebviewWindowBuilder,
};

pub const LABEL: &str = "tray-flyout";
const WIDTH: u32 = 260;
const HEADER_HEIGHT: u32 = 42;
const ROW_HEIGHT: u32 = 40;
const BOTTOM_PADDING: u32 = 10;
const POLL: Duration = Duration::from_millis(50);
const SHOW_DELAY: Duration = Duration::from_millis(200);
const HIDE_DELAY: Duration = Duration::from_millis(300);
const REFRESH: Duration = Duration::from_millis(250);

#[derive(Serialize)]
pub(super) struct Row {
    pub detail: String,
    pub time: String,
    pub progress: f64,
}

#[derive(Serialize)]
struct Panel {
    theme: crate::config::Theme,
    ip: Option<String>,
    pomodoro: Option<Row>,
    work: Option<Row>,
    lunch: Option<Row>,
}

impl Panel {
    fn height(&self) -> u32 {
        let rows = [&self.pomodoro, &self.work, &self.lunch]
            .into_iter()
            .filter(|row| row.is_some())
            .count() as u32;
        HEADER_HEIGHT + rows * ROW_HEIGHT + BOTTOM_PADDING
    }
}

fn snapshot(state: &AppState) -> Option<Panel> {
    let (config, timer) = {
        let runtime = state.runtime.lock().ok()?;
        (runtime.config.clone(), runtime.pomodoro)
    };
    panel_at(&config, timer, Local::now())
}

fn panel_at(
    config: &crate::config::WorkConfig,
    timer: Option<crate::desktop::PomodoroTooltip>,
    now: DateTime<Local>,
) -> Option<Panel> {
    let stats = calculator::calculate(config, now.naive_local()).ok()?;
    let [start, lunch, _, _] = config.times().ok()?;
    let elapsed = now.time().signed_duration_since(start).num_seconds();
    let until_lunch = lunch.signed_duration_since(start).num_seconds();
    Some(Panel {
        theme: config.theme,
        ip: config
            .show_ip
            .then(|| crate::local_ip_suffix().unwrap_or_else(|| "—".into())),
        pomodoro: timer.and_then(|timer| timer.row(now.timestamp_millis())),
        work: (stats.status != "after_work").then(|| Row {
            detail: if stats.status == "lunch_break" {
                "午休中".into()
            } else {
                String::new()
            },
            time: hms(stats.remaining_seconds),
            progress: stats.progress_percent,
        }),
        lunch: (stats.guandan_remaining_seconds > 0).then(|| Row {
            detail: String::new(),
            time: hms(stats.guandan_remaining_seconds),
            progress: (100.0 * elapsed as f64 / until_lunch as f64).clamp(0.0, 100.0),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn movement_recovers_hover_without_restarting_dwell_or_reopening_after_click() {
        let now = Instant::now();
        let icon = Bounds {
            origin: PhysicalPosition::new(100, 100),
            size: PhysicalSize::new(24, 24),
        };
        let mut hover = None;
        Hover::track(&mut hover, icon, now);
        Hover::track(&mut hover, icon, now + Duration::from_millis(150));
        let candidate = hover.as_mut().unwrap();
        assert!(!candidate.expired(false, now + SHOW_DELAY));
        assert!(!candidate.ready(false, now + SHOW_DELAY));
        assert!(candidate.ready(true, now + SHOW_DELAY));
        candidate.dismissed = true;
        Hover::track(&mut hover, icon, now + SHOW_DELAY);
        let candidate = hover.as_ref().unwrap();
        assert!(!candidate.ready(true, now + HIDE_DELAY));
        assert!(candidate.expired(false, now + HIDE_DELAY));
        hover = None;
        Hover::track(&mut hover, icon, now + HIDE_DELAY);
        assert!(hover.unwrap().ready(true, now + HIDE_DELAY + SHOW_DELAY));
    }

    #[test]
    fn lunch_disappears_at_zero_and_work_progress_pauses() {
        let at = |h, m, s| {
            panel_at(
                &crate::config::WorkConfig::default(),
                None,
                Local
                    .with_ymd_and_hms(2026, 11, 27, h, m, s)
                    .single()
                    .unwrap(),
            )
            .unwrap()
        };
        assert_eq!(at(8, 59, 59).lunch.unwrap().progress, 0.0);
        let before = at(11, 44, 59);
        assert_eq!(before.lunch.as_ref().unwrap().time, "00:00:01");
        let lunch_start = at(11, 45, 0);
        let during = at(12, 30, 0);
        assert!(lunch_start.lunch.is_none());
        assert!(during.lunch.is_none());
        assert_eq!(lunch_start.height(), before.height() - ROW_HEIGHT);
        assert_eq!(
            lunch_start.work.unwrap().progress,
            during.work.unwrap().progress
        );
        assert!(at(17, 30, 0).work.is_none());
    }

    #[test]
    fn placement_keeps_panel_inside_scaled_secondary_work_area() {
        let work = Bounds {
            origin: PhysicalPosition::new(-1920, 0),
            size: PhysicalSize::new(1920, 1020),
        };
        let icon = Bounds {
            origin: PhysicalPosition::new(-50, 1040),
            size: PhysicalSize::new(28, 28),
        };
        let panel = placement(icon, work, PhysicalSize::new(390, 258), 12);
        assert_eq!(panel.origin, PhysicalPosition::new(-390, 762));
        let top_icon = Bounds {
            origin: PhysicalPosition::new(-50, 1),
            ..icon
        };
        assert_eq!(placement(top_icon, work, panel.size, 12).origin.y, 41);
    }
}

fn hms(seconds: i64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

#[derive(Clone, Copy, PartialEq)]
struct Bounds {
    origin: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

impl Bounds {
    fn from_tray(rect: Rect) -> Self {
        Self {
            origin: rect.position.to_physical(1.0),
            size: rect.size.to_physical(1.0),
        }
    }

    fn contains(self, point: PhysicalPosition<f64>) -> bool {
        let left = f64::from(self.origin.x);
        let top = f64::from(self.origin.y);
        point.x >= left
            && point.x < left + f64::from(self.size.width)
            && point.y >= top
            && point.y < top + f64::from(self.size.height)
    }
}

fn placement(icon: Bounds, work: Bounds, size: PhysicalSize<u32>, gap: i32) -> Bounds {
    let (left, top) = (work.origin.x, work.origin.y);
    let (right, bottom) = (left + work.size.width as i32, top + work.size.height as i32);
    let (width, height) = (size.width as i32, size.height as i32);
    let (ix, iy) = (icon.origin.x, icon.origin.y);
    let (iw, ih) = (icon.size.width as i32, icon.size.height as i32);
    let (cx, cy) = (ix + iw / 2, iy + ih / 2);
    let (x, y) = if cx < left {
        (ix + iw + gap, cy - height / 2)
    } else if cx >= right {
        (ix - width - gap, cy - height / 2)
    } else if iy - gap - height >= top {
        (cx - width / 2, iy - gap - height)
    } else {
        (cx - width / 2, iy + ih + gap)
    };
    Bounds {
        origin: PhysicalPosition::new(
            x.clamp(left, (right - width).max(left)),
            y.clamp(top, (bottom - height).max(top)),
        ),
        size,
    }
}

struct Hover {
    icon: Bounds,
    entered: Instant,
    last_inside: Instant,
    refreshed: Option<Instant>,
    panel: Option<Bounds>,
    dismissed: bool,
}

impl Hover {
    fn track(slot: &mut Option<Self>, icon: Bounds, now: Instant) {
        if icon.size.width == 0 || icon.size.height == 0 {
            return;
        }
        if let Some(hover) = slot {
            hover.icon = icon;
            hover.last_inside = now;
        } else {
            *slot = Some(Self {
                icon,
                entered: now,
                last_inside: now,
                refreshed: None,
                panel: None,
                dismissed: false,
            });
        }
    }

    fn expired(&self, on_icon: bool, now: Instant) -> bool {
        if self.dismissed {
            !on_icon
        } else {
            now.duration_since(self.last_inside) >= HIDE_DELAY
        }
    }

    fn ready(&self, on_icon: bool, now: Instant) -> bool {
        !self.dismissed
            && (on_icon || self.panel.is_some())
            && now.duration_since(self.entered) >= SHOW_DELAY
            && self
                .refreshed
                .is_none_or(|last| now.duration_since(last) >= REFRESH)
    }
}

#[derive(Default)]
struct HoverState(Mutex<Option<Hover>>);

pub(super) fn setup(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("tray.html".into()))
        .title("我要第一个溜")
        .inner_size(f64::from(WIDTH), 172.0)
        .visible(false)
        .focused(false)
        .focusable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .build()?;
    app.manage(HoverState::default());
    let app = app.handle().clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(POLL);
            let active = app
                .state::<HoverState>()
                .0
                .lock()
                .is_ok_and(|hover| hover.is_some());
            if active {
                let handle = app.clone();
                if app
                    .run_on_main_thread(move || {
                        if let Err(error) = tick(&handle) {
                            eprintln!("托盘悬浮面板更新失败：{error}");
                            if let Ok(mut hover) = handle.state::<HoverState>().0.lock() {
                                *hover = None;
                            }
                            hide(&handle);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }
    });
    Ok(())
}

pub(super) fn track(app: &tauri::AppHandle, rect: Rect) {
    let state = app.state::<HoverState>();
    if let Ok(mut hover) = state.0.lock() {
        Hover::track(&mut hover, Bounds::from_tray(rect), Instant::now());
    }
}

pub(super) fn dismiss(app: &tauri::AppHandle, rect: Rect) {
    if let Some(state) = app.try_state::<HoverState>() {
        if let Ok(mut hover) = state.0.lock() {
            Hover::track(&mut hover, Bounds::from_tray(rect), Instant::now());
            if let Some(hover) = hover.as_mut() {
                hover.dismissed = true;
                hover.panel = None;
            }
        }
    }
    hide(app);
}

pub(super) fn leave(app: &tauri::AppHandle) {
    let state = app.state::<HoverState>();
    if let Ok(mut hover) = state.0.lock() {
        if hover.as_ref().is_some_and(|session| session.dismissed) {
            *hover = None;
        }
    }
}

fn hide(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

fn tick(app: &tauri::AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let state = app.state::<HoverState>();
    let Ok(mut state) = state.0.lock() else {
        return Ok(());
    };
    let Some(hover) = state.as_mut() else {
        return Ok(());
    };
    let now = Instant::now();
    let cursor = window.cursor_position()?;
    let on_icon = hover.icon.contains(cursor);
    if on_icon || hover.panel.is_some_and(|panel| panel.contains(cursor)) {
        hover.last_inside = now;
    } else if hover.expired(on_icon, now) {
        *state = None;
        return window.hide();
    }
    if !hover.ready(on_icon, now) {
        return Ok(());
    }

    let center_x = f64::from(hover.icon.origin.x) + f64::from(hover.icon.size.width) / 2.0;
    let center_y = f64::from(hover.icon.origin.y) + f64::from(hover.icon.size.height) / 2.0;
    let Some(monitor) = window
        .monitor_from_point(center_x, center_y)?
        .or(window.primary_monitor()?)
    else {
        *state = None;
        return window.hide();
    };
    let Some(panel) = snapshot(&app.state::<AppState>()) else {
        return Ok(());
    };
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let bounds = placement(
        hover.icon,
        Bounds {
            origin: work.position,
            size: work.size,
        },
        tauri::LogicalSize::new(WIDTH, panel.height()).to_physical(scale),
        (8.0 * scale).round() as i32,
    );
    if hover.panel != Some(bounds) {
        window.set_position(bounds.origin)?;
        window.set_size(bounds.size)?;
    }
    window.emit("tray-panel", &panel)?;
    if hover.panel.is_none() {
        window.show()?;
    }
    hover.panel = Some(bounds);
    hover.refreshed = Some(now);
    Ok(())
}
