use crate::{AppState, calculator, config, toggle_pin};
use chrono::{Local, NaiveDate, NaiveDateTime, Timelike};
use std::{collections::BTreeSet, fs, time::Duration};
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_notification::NotificationExt;

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
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "visibility", "显示 / 隐藏窗口", true, None::<&str>)?,
            &MenuItem::with_id(app, "pin", "切换窗口置顶", true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", "配置设置", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>)?,
        ],
    )?;
    let tray = TrayIconBuilder::with_id("countdown")
        .icon(tauri::include_image!("icons/tray/32x32.png"))
        .tooltip("我要第一个溜")
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
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => toggle_window(tray.app_handle()),
            TrayIconEvent::Enter { .. } => {
                let app = tray.app_handle();
                update_tooltip(app, &app.state::<AppState>());
            }
            _ => {}
        });
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
    let Ok(runtime) = state.runtime.lock() else {
        return;
    };
    let Ok(stats) = calculator::calculate(&runtime.config, Local::now().naive_local()) else {
        return;
    };
    drop(runtime);
    let text = tooltip_text(&stats);
    if let Some(tray) = app.tray_by_id("countdown") {
        let _ = tray.set_tooltip(Some(text));
    }
}

fn tooltip_text(stats: &calculator::CountdownStats) -> String {
    let mut lines = vec!["我要第一个溜".to_string()];
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
    let guandan = if stats.guandan_ready {
        "掼蛋时间到！".into()
    } else if game == 0 {
        String::new()
    } else {
        format!(
            "掼蛋倒计时 {:02}:{:02}:{:02}",
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
        tooltip_text(&stats)
    }

    #[test]
    fn tooltip_removes_finished_lines_at_exact_boundaries() {
        for (time, expected) in [
            (
                "00:00:00",
                "我要第一个溜\n距离下班 17:30:00\n掼蛋倒计时 11:45:00",
            ),
            (
                "11:44:59",
                "我要第一个溜\n距离下班 05:45:01\n掼蛋倒计时 00:00:01",
            ),
            (
                "11:45:00",
                "我要第一个溜\n距离下班 05:45:00 · 午休中\n掼蛋时间到！",
            ),
            (
                "13:14:59",
                "我要第一个溜\n距离下班 04:15:01 · 午休中\n掼蛋时间到！",
            ),
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
            (
                "13:14:59",
                "我要第一个溜\n距离下班 04:45:01 · 午休中\n掼蛋时间到！",
            ),
            (
                "13:15:00",
                "我要第一个溜\n距离下班 04:45:00 · 午休中\n掼蛋时间到！",
            ),
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
