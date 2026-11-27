use crate::{AppState, calculator, config, flyout, toggle_pin};
use chrono::{Local, NaiveDate, NaiveDateTime, Timelike};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, time::Duration};
use tauri::{
    Emitter, Manager, Wry,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PomodoroPhase {
    Focus,
    Break,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(tag = "kind", content = "milliseconds", rename_all = "snake_case")]
enum PomodoroClock {
    Deadline(i64),
    Paused(i64),
}

#[derive(Clone, Copy, Deserialize)]
pub(super) struct PomodoroTooltip {
    phase: PomodoroPhase,
    clock: PomodoroClock,
    #[serde(default)]
    duration_ms: i64,
}

struct PomodoroMenu(MenuItem<Wry>);

impl PomodoroTooltip {
    fn remaining(self, now_ms: i64) -> (i64, bool) {
        match self.clock {
            PomodoroClock::Deadline(deadline) => (deadline.saturating_sub(now_ms), false),
            PomodoroClock::Paused(remaining) => (remaining, true),
        }
    }

    pub(super) fn row(self, now_ms: i64) -> Option<flyout::Row> {
        let (remaining, paused) = self.remaining(now_ms);
        if remaining <= 0 {
            return None;
        }
        let seconds = (remaining as u64).div_ceil(1000);
        let phase = match self.phase {
            PomodoroPhase::Focus => "专注",
            PomodoroPhase::Break => "休息",
        };
        Some(flyout::Row {
            detail: if paused {
                format!("{phase} · 已暂停")
            } else {
                phase.into()
            },
            time: format!("{:02}:{:02}", seconds / 60, seconds % 60),
            progress: if self.duration_ms > 0 {
                (100.0 * (1.0 - remaining as f64 / self.duration_ms as f64)).clamp(0.0, 100.0)
            } else {
                0.0
            },
        })
    }

    fn line(self, now_ms: i64) -> Option<String> {
        let (remaining, paused) = match self.clock {
            PomodoroClock::Deadline(deadline) => (deadline.saturating_sub(now_ms), false),
            PomodoroClock::Paused(remaining) => (remaining, true),
        };
        if remaining <= 0 {
            return None;
        }
        let seconds = (remaining as u64).div_ceil(1000);
        let phase = match self.phase {
            PomodoroPhase::Focus => "专注",
            PomodoroPhase::Break => "休息",
        };
        let status = if paused { "（已暂停）" } else { "" };
        Some(format!(
            "番茄钟 · {phase} {:02}:{:02}{status}",
            seconds / 60,
            seconds % 60
        ))
    }
}

pub(super) fn update_pomodoro_menu(app: &tauri::AppHandle, timer: Option<PomodoroTooltip>) {
    let Some(menu) = app.try_state::<PomodoroMenu>() else {
        return;
    };
    let label = match timer.map(|timer| timer.clock) {
        Some(PomodoroClock::Deadline(_)) => "暂停番茄钟",
        Some(PomodoroClock::Paused(_)) => "继续番茄钟",
        None => "开始番茄钟",
    };
    let _ = menu.0.set_text(label);
}

pub fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.emit("widget-shown", ());
    }
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_window(app);
        }
    }
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let pomodoro = MenuItem::with_id(app, "pomodoro", "开始番茄钟", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "visibility", "显示 / 隐藏窗口", true, None::<&str>)?,
            &MenuItem::with_id(app, "pin", "切换窗口置顶", true, None::<&str>)?,
            &MenuItem::with_id(app, "mini", "切换迷你模式", true, None::<&str>)?,
            &pomodoro,
            &MenuItem::with_id(app, "settings", "配置设置", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>)?,
        ],
    )?;
    app.manage(PomodoroMenu(pomodoro));
    let custom_hover = match flyout::setup(app) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("托盘悬浮面板创建失败：{error}");
            false
        }
    };
    let tray = TrayIconBuilder::with_id("countdown")
        .icon(tauri::include_image!("icons/tray/32x32.png"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "visibility" => toggle_window(app),
            "pin" => {
                if let Err(error) = toggle_pin(app.clone(), app.state::<AppState>()) {
                    let _ = app.emit("app-error", error);
                }
            }
            "settings" => {
                show_window(app);
                let _ = app.emit("open-settings", ());
            }
            "mini" => {
                let _ = app.emit("toggle-mini-mode", ());
            }
            "pomodoro" => {
                let _ = app.emit("toggle-pomodoro", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } => {
                flyout::dismiss(tray.app_handle(), rect);
                toggle_window(tray.app_handle());
            }
            TrayIconEvent::Click { rect, .. } => flyout::dismiss(tray.app_handle(), rect),
            TrayIconEvent::Enter { rect, .. } | TrayIconEvent::Move { rect, .. }
                if custom_hover =>
            {
                flyout::track(tray.app_handle(), rect)
            }
            TrayIconEvent::Leave { .. } if custom_hover => flyout::leave(tray.app_handle()),
            TrayIconEvent::Enter { .. } => {
                let app = tray.app_handle();
                update_tooltip(app, &app.state::<AppState>());
            }
            _ => {}
        });
    let tray = if custom_hover {
        tray
    } else {
        tray.tooltip("我要第一个溜")
    };
    tray.build(app)?;
    update_tooltip(app.handle(), &app.state::<AppState>());
    Ok(())
}

fn reminder_due(
    now: NaiveDateTime,
    sent: &BTreeSet<NaiveDate>,
    config: &config::WorkConfig,
) -> bool {
    config.reminder_enabled
        && config
            .reminder_time()
            .is_ok_and(|time| now.hour() == time.hour() && now.minute() == time.minute())
        && !sent.contains(&now.date())
}

pub fn start_clock(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let marker_path = state.directory.join("reminders.json");
        let records = match fs::read(&marker_path) {
            Ok(bytes) => {
                serde_json::from_slice::<BTreeSet<NaiveDate>>(&bytes).map_err(|e| e.to_string())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
            Err(error) => Err(error.to_string()),
        };
        let reminders_enabled = records.is_ok();
        if let Err(error) = &records {
            if let Ok(mut runtime) = state.runtime.lock() {
                runtime
                    .warnings
                    .push(format!("提醒记录无法读取，外卖提醒已暂停：{error}"));
            }
        }
        let mut sent = records.unwrap_or_default();
        loop {
            update_tooltip(&app, &state);
            let now = Local::now().naive_local();
            let due = reminders_enabled
                && state
                    .runtime
                    .lock()
                    .map(|runtime| reminder_due(now, &sent, &runtime.config))
                    .unwrap_or(false);
            if due {
                // Persist before dispatch so a restart cannot send twice.
                sent.insert(now.date());
                match config::write_json(&marker_path, &sent) {
                    Ok(()) => {
                        if let Err(error) = app
                            .notification()
                            .builder()
                            .title("该点外卖啦")
                            .body("先把午饭安排好，忙碌之余也要好好吃饭。")
                            .show()
                        {
                            let _ = app.emit("app-error", format!("外卖提醒发送失败：{error}"));
                        }
                    }
                    Err(error) => {
                        let _ = app.emit(
                            "app-error",
                            format!("提醒记录保存失败，本次未发送：{error}"),
                        );
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

pub(super) fn update_tooltip(app: &tauri::AppHandle, state: &AppState) {
    let Ok(mut runtime) = state.runtime.lock() else {
        return;
    };
    let now = Local::now();
    let timer = runtime.pomodoro;
    let expired = timer.is_some_and(|timer| timer.remaining(now.timestamp_millis()).0 <= 0);
    if expired {
        runtime.pomodoro = None;
    }
    let fallback = app.get_webview_window(flyout::LABEL).is_none();
    let stats = fallback
        .then(|| calculator::calculate(&runtime.config, now.naive_local()).ok())
        .flatten();
    drop(runtime);
    if expired {
        update_pomodoro_menu(app, None);
    }
    let Some(stats) = stats else {
        return;
    };
    let pomodoro = timer.and_then(|timer| timer.line(now.timestamp_millis()));
    let suffix = if stats.schedule.show_ip {
        crate::local_ip_suffix()
    } else {
        None
    };
    let text = tooltip_text(&stats, suffix.as_deref(), pomodoro.as_deref());
    if let Some(tray) = app.tray_by_id("countdown") {
        let _ = tray.set_tooltip(Some(text));
    }
}

fn tooltip_text(
    stats: &calculator::CountdownStats,
    suffix: Option<&str>,
    pomodoro: Option<&str>,
) -> String {
    let title = if stats.schedule.show_ip {
        format!("我要第一个溜 {}", suffix.unwrap_or("—"))
    } else {
        "我要第一个溜".to_string()
    };
    let mut lines = vec![title];
    if let Some(line) = pomodoro {
        lines.push(line.to_string());
    }
    let seconds = stats.remaining_seconds;
    let text = match stats.status {
        "after_work" => String::new(),
        _ => format!(
            "距离下班 {:02}:{:02}:{:02}{}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60,
            if stats.status == "lunch_break" {
                " · 午休中"
            } else {
                ""
            }
        ),
    };
    if !text.is_empty() {
        lines.push(text);
    }
    let game = stats.guandan_remaining_seconds;
    let guandan = if game == 0 {
        String::new()
    } else {
        format!(
            "开饭倒计时 {:02}:{:02}:{:02}",
            game / 3600,
            game / 60 % 60,
            game % 60
        )
    };
    if !guandan.is_empty() {
        lines.push(guandan);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tooltip(config: &config::WorkConfig, timestamp: &str) -> String {
        let stats = calculator::calculate(
            config,
            NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S").unwrap(),
        )
        .unwrap();
        tooltip_text(&stats, None, None)
    }

    #[test]
    fn tooltip_ip_follows_display_setting_even_after_work() {
        let mut config = config::WorkConfig::default();
        let now =
            NaiveDateTime::parse_from_str("2026-11-13 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let stats = calculator::calculate(&config, now).unwrap();
        assert_eq!(tooltip_text(&stats, Some("123"), None), "我要第一个溜");
        config.show_ip = true;
        let stats = calculator::calculate(&config, now).unwrap();
        assert_eq!(tooltip_text(&stats, Some("123"), None), "我要第一个溜 123");
        assert_eq!(tooltip_text(&stats, None, None), "我要第一个溜 —");
    }

    #[test]
    fn pomodoro_tooltip_counts_down_and_keeps_paused_time() {
        let running = PomodoroTooltip {
            phase: PomodoroPhase::Focus,
            clock: PomodoroClock::Deadline(61_000),
            duration_ms: 61_000,
        };
        assert_eq!(running.line(0).as_deref(), Some("番茄钟 · 专注 01:01"));
        assert_eq!(running.line(60_999).as_deref(), Some("番茄钟 · 专注 00:01"));
        assert_eq!(running.line(61_000), None);
        assert_eq!(running.row(30_500).unwrap().progress, 50.0);

        let paused: PomodoroTooltip = serde_json::from_value(serde_json::json!({
            "phase": "break",
            "clock": { "kind": "paused", "milliseconds": 25_100 }
        }))
        .unwrap();
        let line = paused.line(99_000);
        assert_eq!(line.as_deref(), Some("番茄钟 · 休息 00:26（已暂停）"));
        let stats = calculator::calculate(
            &config::WorkConfig::default(),
            NaiveDateTime::parse_from_str("2026-11-13 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(
            tooltip_text(&stats, None, line.as_deref()),
            "我要第一个溜\n番茄钟 · 休息 00:26（已暂停）"
        );
    }

    #[test]
    fn tooltip_removes_finished_lines_at_exact_boundaries() {
        for (time, expected) in [
            (
                "00:00:00",
                "我要第一个溜\n距离下班 17:30:00\n开饭倒计时 11:45:00",
            ),
            (
                "11:44:59",
                "我要第一个溜\n距离下班 05:45:01\n开饭倒计时 00:00:01",
            ),
            ("11:45:00", "我要第一个溜\n距离下班 05:45:00 · 午休中"),
            ("13:14:59", "我要第一个溜\n距离下班 04:15:01 · 午休中"),
            ("13:15:00", "我要第一个溜\n距离下班 04:15:00"),
            ("17:29:59", "我要第一个溜\n距离下班 00:00:01"),
            ("17:30:00", "我要第一个溜"),
            ("23:59:59", "我要第一个溜"),
        ] {
            let actual = tooltip(
                &config::WorkConfig::default(),
                &format!("2026-11-11 {time}"),
            );
            assert_eq!(actual, expected, "{time}");
            println!("{time}: {actual:?}");
        }
    }

    #[test]
    fn tooltip_uses_configured_end_and_lunch_interval() {
        let config = config::WorkConfig {
            lunch_start_time: "12:00".into(),
            lunch_end_time: "14:00".into(),
            end_time: "18:00".into(),
            ..config::WorkConfig::default()
        };
        for (time, expected) in [
            ("13:14:59", "我要第一个溜\n距离下班 04:45:01 · 午休中"),
            ("13:15:00", "我要第一个溜\n距离下班 04:45:00 · 午休中"),
            ("14:00:00", "我要第一个溜\n距离下班 04:00:00"),
            ("17:30:00", "我要第一个溜\n距离下班 00:30:00"),
            ("17:59:59", "我要第一个溜\n距离下班 00:00:01"),
            ("18:00:00", "我要第一个溜"),
        ] {
            assert_eq!(
                tooltip(&config, &format!("2026-11-11 {time}")),
                expected,
                "{time}"
            );
        }
        let early = config::WorkConfig {
            start_time: "08:00".into(),
            lunch_start_time: "10:00".into(),
            lunch_end_time: "11:00".into(),
            end_time: "12:00".into(),
            ..config::WorkConfig::default()
        };
        assert_eq!(
            tooltip(&early, "2026-11-11 11:59:59"),
            "我要第一个溜\n距离下班 00:00:01"
        );
        assert_eq!(tooltip(&early, "2026-11-11 12:00:00"), "我要第一个溜");
        assert_eq!(tooltip(&early, "2026-11-11 13:15:00"), "我要第一个溜");
    }

    #[test]
    fn tooltip_uses_same_schedule_on_every_date() {
        let config = config::WorkConfig::default();
        for date in ["2026-11-21", "2026-11-29", "2026-10-01"] {
            for time in ["13:14:59", "13:15:00", "17:29:59", "17:30:00"] {
                assert_eq!(
                    tooltip(&config, &format!("{date} {time}")),
                    tooltip(&config, &format!("2026-11-11 {time}"))
                );
            }
        }
        assert_eq!(
            tooltip(&config, "2026-11-28 17:29:59"),
            "我要第一个溜\n距离下班 00:00:01"
        );
        assert_eq!(tooltip(&config, "2026-11-28 17:30:00"), "我要第一个溜");
    }

    #[test]
    fn reminder_switch_custom_time_and_same_day_changes() {
        let mut config = config::WorkConfig {
            reminder_time: "12:20".into(),
            ..config::WorkConfig::default()
        };
        let mut sent = BTreeSet::new();
        let at = |time: &str| {
            NaiveDateTime::parse_from_str(&format!("2026-11-12 {time}"), "%Y-%m-%d %H:%M:%S")
                .unwrap()
        };
        assert!(!reminder_due(at("10:35:00"), &sent, &config));
        assert!(!reminder_due(at("12:19:59"), &sent, &config));
        assert!(reminder_due(at("12:20:00"), &sent, &config));
        assert!(reminder_due(at("12:20:59"), &sent, &config));
        assert!(!reminder_due(at("12:21:00"), &sent, &config));
        config.reminder_enabled = false;
        assert!(!reminder_due(at("12:20:00"), &sent, &config));
        config.reminder_enabled = true;
        sent.insert(at("12:20:00").date());
        config.reminder_time = "12:25".into();
        assert!(!reminder_due(at("12:25:00"), &sent, &config));
        let tomorrow = at("12:25:00") + chrono::Duration::days(1);
        assert!(reminder_due(tomorrow, &sent, &config));
    }

    #[test]
    fn reminder_target_minute_and_daily_deduplication() {
        let config = config::WorkConfig::default();
        let mut sent = BTreeSet::new();
        let now =
            NaiveDateTime::parse_from_str("2026-11-11 10:35:59", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(reminder_due(now, &sent, &config));
        assert!(!reminder_due(
            now + chrono::Duration::seconds(1),
            &sent,
            &config
        ));
        sent.insert(now.date());
        assert!(!reminder_due(now, &sent, &config));
        let holiday = NaiveDate::from_ymd_opt(2026, 10, 1)
            .unwrap()
            .and_hms_opt(10, 35, 0)
            .unwrap();
        assert!(reminder_due(holiday, &sent, &config));
        for date in ["2026-11-29", "2027-01-01"] {
            let now =
                NaiveDateTime::parse_from_str(&format!("{date} 10:35:00"), "%Y-%m-%d %H:%M:%S")
                    .unwrap();
            assert!(reminder_due(now, &sent, &config));
        }
    }
}
