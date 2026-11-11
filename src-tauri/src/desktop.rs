use crate::{AppState, calculator, config, toggle_pin};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Timelike};
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
    calendar: &calculator::HolidayCalendar,
    sent: &BTreeSet<NaiveDate>,
) -> bool {
    now.hour() == 10
        && now.minute() == 35
        && calendar.years.contains(&now.year())
        && calculator::is_workday(now.date(), calendar)
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
            if reminders_enabled && reminder_due(now, &state.calendar, &sent) {
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
    let Ok(stats) =
        calculator::calculate(&runtime.config, Local::now().naive_local(), &state.calendar)
    else {
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
        "day_off" => "下班倒计时 · 休息日 / 充能中".into(),
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
    let guandan = if !stats.is_workday {
        "掼蛋倒计时 · 休息日".into()
    } else if stats.guandan_ready {
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
            &calculator::HolidayCalendar::bundled(),
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
    fn tooltip_uses_configured_end_and_fixed_guandan_cutoff() {
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
            ("13:15:00", "我要第一个溜\n距离下班 04:45:00 · 午休中"),
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
            "我要第一个溜\n距离下班 00:00:01\n掼蛋时间到！"
        );
        assert_eq!(
            tooltip(&early, "2026-11-11 12:00:00"),
            "我要第一个溜\n掼蛋时间到！"
        );
        assert_eq!(tooltip(&early, "2026-11-11 13:15:00"), "我要第一个溜");
    }

    #[test]
    fn tooltip_preserves_rest_days_holidays_and_last_saturday() {
        let config = config::WorkConfig::default();
        for date in ["2026-11-21", "2026-11-29", "2026-10-01"] {
            for time in ["13:14:59", "13:15:00", "17:29:59", "17:30:00"] {
                assert_eq!(
                    tooltip(&config, &format!("{date} {time}")),
                    "我要第一个溜\n下班倒计时 · 休息日 / 充能中\n掼蛋倒计时 · 休息日"
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
    fn reminder_target_minute_workday_and_deduplication() {
        let calendar = calculator::HolidayCalendar::bundled();
        let mut sent = BTreeSet::new();
        let now =
            NaiveDateTime::parse_from_str("2026-11-11 10:35:59", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(reminder_due(now, &calendar, &sent));
        assert!(!reminder_due(
            now + chrono::Duration::seconds(1),
            &calendar,
            &sent
        ));
        sent.insert(now.date());
        assert!(!reminder_due(now, &calendar, &sent));
        let holiday = NaiveDate::from_ymd_opt(2026, 10, 1)
            .unwrap()
            .and_hms_opt(10, 35, 0)
            .unwrap();
        assert!(!reminder_due(holiday, &calendar, &sent));
    }
}
